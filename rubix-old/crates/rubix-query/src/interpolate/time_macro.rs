//! Lower the time macros into bound parameters + portable SQL.
//!
//! These are the `$__`-prefixed temporal tokens the dashboard time-range picker
//! relies on (docs/design/time-range-and-refresh.md §4). Like every other
//! token the engine handles, the *values* (the resolved range bounds and the
//! bucket width) leave as bound parameters — only the operator-authored column
//! identifier, which is the same trust tier as the rest of the widget SQL,
//! splices into the text:
//!   - `$__from`                         → `$N` bound to the resolved lower bound.
//!   - `$__to`                           → `$N` bound to the resolved upper bound.
//!   - `$__interval`                     → `$N` bound to the resolved seconds (int).
//!   - `$__timeFilter(col)`              → `col >= $N AND col < $M`.
//!   - `$__timeGroup(col, '$__interval')`→ epoch-floor bucketing by the interval.
//!
//! A time macro with no `time_range` on the request is a hard error, never a
//! silent passthrough that would leave a raw `$__` token in the SQL.

use super::bound::BoundParam;
use super::error::InterpolateError;
use super::time::TimeContext;

/// The result of matching one time macro at a byte offset: the SQL fragment to
/// emit, the parameters it bound, and the byte index just past the macro.
pub(super) struct TimeMatch {
    /// SQL text to append in place of the macro (with `$N` placeholders).
    pub sql: String,
    /// Parameters bound by this macro, in placeholder order.
    pub params: Vec<BoundParam>,
    /// Byte index in the source SQL just past the consumed macro.
    pub end: usize,
}

/// If `sql[at..]` opens a time macro, lower it. Returns `Ok(None)` when no time
/// macro starts here (so the caller falls through to the variable tokens).
///
/// `next` is the next free `$N` index; this advances it past the placeholders
/// it emits. `time` is the resolved range; a time macro with `time == None` is
/// a [`InterpolateError::MissingTimeRange`].
pub(super) fn lower_time_macro(
    sql: &str,
    at: usize,
    next: &mut usize,
    time: Option<&TimeContext>,
) -> Result<Option<TimeMatch>, InterpolateError> {
    let rest = &sql[at..];
    if rest.starts_with("$__timeFilter(") {
        return lower_time_filter(sql, at, next, time).map(Some);
    }
    if rest.starts_with("$__timeGroup(") {
        return lower_time_group(sql, at, next, time).map(Some);
    }
    // The bare scalar macros. Longest-match first so `$__from` is not shadowed.
    if rest.starts_with("$__from") {
        let ctx = require(time, "$__from")?;
        return Ok(Some(scalar_match(
            next,
            BoundParam::Timestamp(ctx.lower_rfc3339()),
            at + "$__from".len(),
        )));
    }
    if rest.starts_with("$__to") {
        let ctx = require(time, "$__to")?;
        return Ok(Some(scalar_match(
            next,
            BoundParam::Timestamp(ctx.upper_rfc3339()),
            at + "$__to".len(),
        )));
    }
    if rest.starts_with("$__interval") {
        let ctx = require(time, "$__interval")?;
        return Ok(Some(scalar_match(
            next,
            BoundParam::Int(i64::from(ctx.interval_secs)),
            at + "$__interval".len(),
        )));
    }
    Ok(None)
}

/// Emit a single `$N` placeholder bound to `param`.
fn scalar_match(next: &mut usize, param: BoundParam, end: usize) -> TimeMatch {
    let sql = format!("${next}");
    *next += 1;
    TimeMatch {
        sql,
        params: vec![param],
        end,
    }
}

/// `$__timeFilter(col)` → `col >= $N AND col < $M` (lower inclusive, upper
/// exclusive — matching the `his` `start`/`end` semantics).
fn lower_time_filter(
    sql: &str,
    at: usize,
    next: &mut usize,
    time: Option<&TimeContext>,
) -> Result<TimeMatch, InterpolateError> {
    let ctx = require(time, "$__timeFilter")?;
    let (col, end) = parse_one_arg(sql, at, "$__timeFilter(")?;
    let from_idx = *next;
    let to_idx = *next + 1;
    *next += 2;
    let out = format!("{col} >= ${from_idx} AND {col} < ${to_idx}");
    Ok(TimeMatch {
        sql: out,
        params: vec![
            BoundParam::Timestamp(ctx.lower_rfc3339()),
            BoundParam::Timestamp(ctx.upper_rfc3339()),
        ],
        end,
    })
}

/// `$__timeGroup(col, '$__interval')` → `FLOOR(EXTRACT(EPOCH FROM
/// CAST(col AS TIMESTAMP)) / $N) * $N`, the bucket floor in epoch seconds. The
/// second argument is the interval; it is accepted as the literal
/// `'$__interval'` placeholder or an explicit seconds integer, but the bound
/// width is always the resolved [`TimeContext::interval_secs`] (the resolved
/// interval is what binds — docs/design/time-range-and-refresh.md §4). The
/// `FLOOR(EXTRACT(EPOCH …))` form is valid on both DataFusion and Timescale.
fn lower_time_group(
    sql: &str,
    at: usize,
    next: &mut usize,
    time: Option<&TimeContext>,
) -> Result<TimeMatch, InterpolateError> {
    let ctx = require(time, "$__timeGroup")?;
    let (args, end) = parse_args(sql, at, "$__timeGroup(")?;
    // The column is the first argument; the interval argument is informational
    // (the resolved width binds), so a missing second arg is the only error.
    let mut parts = args.splitn(2, ',');
    let col = parts.next().unwrap_or_default().trim();
    if col.is_empty() || parts.next().is_none() {
        return Err(InterpolateError::MalformedTimeMacro {
            near: sql[at..].chars().take(24).collect(),
        });
    }
    let idx = *next;
    *next += 1;
    let out =
        format!("FLOOR(EXTRACT(EPOCH FROM CAST({col} AS TIMESTAMP)) / ${idx}) * ${idx}");
    Ok(TimeMatch {
        sql: out,
        params: vec![BoundParam::Int(i64::from(ctx.interval_secs))],
        end,
    })
}

/// Require a resolved range for a macro that needs one.
fn require<'a>(
    time: Option<&'a TimeContext>,
    macro_name: &str,
) -> Result<&'a TimeContext, InterpolateError> {
    time.ok_or_else(|| InterpolateError::MissingTimeRange {
        macro_name: macro_name.to_string(),
    })
}

/// Parse a single-argument macro `prefix col )`, returning the trimmed column
/// and the byte index past the `)`.
fn parse_one_arg(
    sql: &str,
    at: usize,
    prefix: &str,
) -> Result<(String, usize), InterpolateError> {
    let (args, end) = parse_args(sql, at, prefix)?;
    let col = args.trim();
    if col.is_empty() || col.contains(',') {
        return Err(InterpolateError::MalformedTimeMacro {
            near: sql[at..].chars().take(24).collect(),
        });
    }
    Ok((col.to_string(), end))
}

/// Parse a parenthesised argument list `prefix … )`, returning the raw inner
/// text and the byte index past the `)`.
fn parse_args(sql: &str, at: usize, prefix: &str) -> Result<(String, usize), InterpolateError> {
    let inner_start = at + prefix.len();
    let rest = &sql[inner_start..];
    match rest.find(')') {
        Some(rel) => Ok((rest[..rel].to_string(), inner_start + rel + 1)),
        None => Err(InterpolateError::Unterminated {
            near: sql[at..].chars().take(24).collect(),
        }),
    }
}
