//! Lower variable tokens in SQL text into `$N` placeholders + bound parameters.
//!
//! This is the injection boundary (docs/design/variables-and-templating.md §2).
//! Every variable value becomes a bound parameter appended to the parameter
//! list; the SQL text only ever gains `$N` placeholders. A value of
//! `'); DROP TABLE points; --` is bound as a literal string and can never
//! execute as SQL. Quoting/escaping does not exist here because nothing is
//! quoted or escaped — values never touch the SQL text at all.
//!
//! Recognised tokens, longest-match first so `$__sqlIn(` is never mistaken for a
//! bare `$name`:
//!   - `$__sqlIn(name)`     → `IN ($1, $2, …)` over a variable's values.
//!   - `${name:csv}`        → `$1, $2, …` (comma-joined, each value bound).
//!   - `${name:singlequote}`→ `$1, $2, …` (same bound expansion; the "quote" is
//!     the driver's, since the engine binds rather than splices).
//!   - `${name}`            → `$1` (single value).
//!   - `$name`              → `$1` (single value, bare form).
//!
//! An existing positional placeholder (`$1`, `$2`, …) is left untouched.

use super::bound::{BoundParam, Lowered};
use super::error::InterpolateError;
use super::time::TimeContext;
use super::time_macro::lower_time_macro;
use super::var::{QueryVariable, Scalar, VarValue};

/// Lower every variable token in `sql` into bound parameters.
///
/// `start_index` is the count of placeholders already present in `sql` from a
/// pre-existing positional parameter list (the datasource path's `params`); the
/// engine numbers its placeholders from `start_index + 1` so the two lists
/// concatenate without collision. The DataFusion path passes `0`.
///
/// Returns the rewritten SQL and the bound parameters in placeholder order.
///
/// Variable-only callers pass `None` for `time`; a `$__from`/`$__timeFilter`/…
/// macro then errors rather than leaving an unbound token. A resolved
/// [`TimeContext`] enables the time macros (docs/design/time-range-and-refresh.md
/// §4); its values bind, never splice.
pub fn lower(
    sql: &str,
    variables: &[QueryVariable],
    start_index: usize,
    time: Option<&TimeContext>,
) -> Result<Lowered, InterpolateError> {
    let mut out = String::with_capacity(sql.len());
    let mut params: Vec<BoundParam> = Vec::new();
    let mut next = start_index + 1;
    let bytes = sql.as_bytes();
    let mut i = 0;

    while i < sql.len() {
        if bytes[i] != b'$' {
            // Copy this UTF-8 char verbatim; advance by its byte length.
            let ch = sql[i..].chars().next().expect("char boundary");
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }

        // A positional placeholder (`$1`, `$12`) is already a bound parameter;
        // leave it as-is so a caller may mix `params` with variables.
        if let Some(end) = positional_end(sql, i) {
            out.push_str(&sql[i..end]);
            i = end;
            continue;
        }

        // Time macros (`$__from`, `$__timeFilter(col)`, …) take precedence over
        // the variable tokens: they share the `$__` prefix with `$__sqlIn` but
        // bind range bounds rather than a named variable's values.
        if let Some(m) = lower_time_macro(sql, i, &mut next, time)? {
            out.push_str(&m.sql);
            params.extend(m.params);
            i = m.end;
            continue;
        }

        if let Some((name, end)) = parse_sql_in(sql, i)? {
            let values = lookup(variables, &name)?.scalars();
            if values.is_empty() {
                return Err(InterpolateError::EmptyExpansion { name });
            }
            out.push_str("IN (");
            emit_list(&mut out, &mut params, &mut next, values);
            out.push(')');
            i = end;
            continue;
        }

        if let Some((name, format, end)) = parse_brace(sql, i)? {
            let value = lookup(variables, &name)?;
            match format.as_deref() {
                None => emit_single(&mut out, &mut params, &mut next, &name, value)?,
                Some("csv") | Some("singlequote") => {
                    let values = value.scalars();
                    if values.is_empty() {
                        return Err(InterpolateError::EmptyExpansion { name });
                    }
                    emit_list(&mut out, &mut params, &mut next, values);
                }
                Some(other) => {
                    return Err(InterpolateError::UnknownFormat {
                        name,
                        format: other.to_string(),
                    })
                }
            }
            i = end;
            continue;
        }

        if let Some((name, end)) = parse_bare(sql, i) {
            let value = lookup(variables, &name)?;
            emit_single(&mut out, &mut params, &mut next, &name, value)?;
            i = end;
            continue;
        }

        // A lone `$` that opens no recognised token (e.g. `$` in a string the
        // caller wrote). Copy it verbatim; it binds nothing.
        out.push('$');
        i += 1;
    }

    Ok(Lowered { sql: out, params })
}

/// Find the supplied value for `name`, or an unknown-variable error.
fn lookup<'a>(
    variables: &'a [QueryVariable],
    name: &str,
) -> Result<&'a VarValue, InterpolateError> {
    variables
        .iter()
        .find(|v| v.name == name)
        .map(|v| &v.value)
        .ok_or_else(|| InterpolateError::UnknownVariable {
            name: name.to_string(),
        })
}

/// Emit one `$N` placeholder for a single-valued token; reject a multi value.
fn emit_single(
    out: &mut String,
    params: &mut Vec<BoundParam>,
    next: &mut usize,
    name: &str,
    value: &VarValue,
) -> Result<(), InterpolateError> {
    let scalar = match value {
        VarValue::One(s) => s,
        VarValue::Many(values) if values.len() == 1 => &values[0],
        VarValue::Many(_) => {
            return Err(InterpolateError::MultiValueInSingle {
                name: name.to_string(),
            })
        }
    };
    push_param(out, params, next, scalar);
    Ok(())
}

/// Emit a comma-joined list of `$N` placeholders, one per value.
fn emit_list(
    out: &mut String,
    params: &mut Vec<BoundParam>,
    next: &mut usize,
    values: &[Scalar],
) {
    for (idx, scalar) in values.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        push_param(out, params, next, scalar);
    }
}

/// Append one bound parameter and write its `$N` placeholder.
fn push_param(out: &mut String, params: &mut Vec<BoundParam>, next: &mut usize, scalar: &Scalar) {
    out.push('$');
    out.push_str(&next.to_string());
    *next += 1;
    params.push(BoundParam::from(scalar));
}

/// If `sql[at..]` is `$<digits>`, return the byte index just past the digits.
fn positional_end(sql: &str, at: usize) -> Option<usize> {
    let rest = &sql[at + 1..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then_some(at + 1 + digits)
}

/// If `sql[at..]` opens `$__sqlIn(name)`, return `(name, end)` past the `)`.
fn parse_sql_in(sql: &str, at: usize) -> Result<Option<(String, usize)>, InterpolateError> {
    const PREFIX: &str = "$__sqlIn(";
    if !sql[at..].starts_with(PREFIX) {
        return Ok(None);
    }
    let name_start = at + PREFIX.len();
    let rest = &sql[name_start..];
    match rest.find(')') {
        Some(rel) => {
            let name = rest[..rel].trim().to_string();
            Ok(Some((name, name_start + rel + 1)))
        }
        None => Err(InterpolateError::Unterminated {
            near: sql[at..].chars().take(16).collect(),
        }),
    }
}

/// If `sql[at..]` opens `${name}` / `${name:format}`, return `(name, format,
/// end)` past the `}`.
#[allow(clippy::type_complexity)]
fn parse_brace(
    sql: &str,
    at: usize,
) -> Result<Option<(String, Option<String>, usize)>, InterpolateError> {
    if !sql[at..].starts_with("${") {
        return Ok(None);
    }
    let inner_start = at + 2;
    let rest = &sql[inner_start..];
    let rel = rest.find('}').ok_or_else(|| InterpolateError::Unterminated {
        near: sql[at..].chars().take(16).collect(),
    })?;
    let inner = &rest[..rel];
    let (name, format) = match inner.split_once(':') {
        Some((n, f)) => (n.trim().to_string(), Some(f.trim().to_string())),
        None => (inner.trim().to_string(), None),
    };
    Ok(Some((name, format, inner_start + rel + 1)))
}

/// If `sql[at..]` opens a bare `$name`, return `(name, end)` past the name. A
/// name is `[A-Za-z_][A-Za-z0-9_]*`; anything else is not a variable token.
fn parse_bare(sql: &str, at: usize) -> Option<(String, usize)> {
    let rest = &sql[at + 1..];
    let mut chars = rest.char_indices();
    let first = chars.next()?;
    if !(first.1.is_ascii_alphabetic() || first.1 == '_') {
        return None;
    }
    let mut end_rel = first.1.len_utf8();
    for (offset, ch) in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end_rel = offset + ch.len_utf8();
        } else {
            break;
        }
    }
    let name = rest[..end_rel].to_string();
    Some((name, at + 1 + end_rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(name: &str, value: &str) -> QueryVariable {
        QueryVariable {
            name: name.to_string(),
            value: VarValue::One(Scalar::Text(value.to_string())),
        }
    }

    fn many(name: &str, values: &[&str]) -> QueryVariable {
        QueryVariable {
            name: name.to_string(),
            value: VarValue::Many(values.iter().map(|v| Scalar::Text(v.to_string())).collect()),
        }
    }

    fn time_ctx() -> TimeContext {
        use chrono::{TimeZone, Utc};
        TimeContext {
            from: Utc.with_ymd_and_hms(2026, 6, 13, 0, 0, 0).single().unwrap(),
            to: Utc.with_ymd_and_hms(2026, 6, 13, 6, 0, 0).single().unwrap(),
            interval_secs: 60,
        }
    }

    #[test]
    fn from_and_to_macros_bind_resolved_bounds_as_timestamps() {
        let t = time_ctx();
        let out = lower("WHERE ts >= $__from AND ts < $__to", &[], 0, Some(&t)).unwrap();
        assert_eq!(out.sql, "WHERE ts >= $1 AND ts < $2");
        assert_eq!(
            out.params,
            vec![
                BoundParam::Timestamp(t.lower_rfc3339()),
                BoundParam::Timestamp(t.upper_rfc3339()),
            ]
        );
    }

    #[test]
    fn time_filter_expands_to_half_open_range_with_bound_bounds() {
        let t = time_ctx();
        let out = lower("WHERE $__timeFilter(ts)", &[], 0, Some(&t)).unwrap();
        assert_eq!(out.sql, "WHERE ts >= $1 AND ts < $2");
        assert_eq!(out.params.len(), 2);
        assert!(matches!(out.params[0], BoundParam::Timestamp(_)));
        // The column identifier is the only spliced text; no instant string is.
        assert!(!out.sql.contains("2026"));
    }

    #[test]
    fn time_group_buckets_by_bound_interval() {
        let t = time_ctx();
        let out = lower(
            "SELECT $__timeGroup(ts, '$__interval') AS b",
            &[],
            0,
            Some(&t),
        )
        .unwrap();
        assert_eq!(
            out.sql,
            "SELECT FLOOR(EXTRACT(EPOCH FROM CAST(ts AS TIMESTAMP)) / $1) * $1 AS b"
        );
        assert_eq!(out.params, vec![BoundParam::Int(60)]);
    }

    #[test]
    fn interval_macro_binds_resolved_seconds() {
        let t = time_ctx();
        let out = lower("SELECT $__interval", &[], 0, Some(&t)).unwrap();
        assert_eq!(out.sql, "SELECT $1");
        assert_eq!(out.params, vec![BoundParam::Int(60)]);
    }

    #[test]
    fn time_macro_with_no_range_is_an_error() {
        let err = lower("WHERE $__timeFilter(ts)", &[], 0, None).unwrap_err();
        assert!(matches!(err, InterpolateError::MissingTimeRange { .. }));
    }

    #[test]
    fn time_and_variable_macros_number_in_one_sequence() {
        let t = time_ctx();
        let vars = vec![one("site", "A")];
        let out = lower(
            "WHERE site = $site AND $__timeFilter(ts)",
            &vars,
            0,
            Some(&t),
        )
        .unwrap();
        assert_eq!(out.sql, "WHERE site = $1 AND ts >= $2 AND ts < $3");
        assert_eq!(out.params.len(), 3);
    }

    #[test]
    fn no_time_macro_query_is_unaffected_by_a_supplied_range() {
        let t = time_ctx();
        let out = lower("SELECT 1", &[], 0, Some(&t)).unwrap();
        assert_eq!(out.sql, "SELECT 1");
        assert!(out.params.is_empty());
    }

    #[test]
    fn malformed_time_filter_is_an_error() {
        let t = time_ctx();
        let err = lower("WHERE $__timeFilter()", &[], 0, Some(&t)).unwrap_err();
        assert!(matches!(err, InterpolateError::MalformedTimeMacro { .. }));
    }

    #[test]
    fn time_group_missing_interval_arg_is_an_error() {
        let t = time_ctx();
        let err = lower("SELECT $__timeGroup(ts)", &[], 0, Some(&t)).unwrap_err();
        assert!(matches!(err, InterpolateError::MalformedTimeMacro { .. }));
    }

    #[test]
    fn bare_token_binds_single_value() {
        let vars = vec![one("site", "Site-A")];
        let out = lower("SELECT * FROM points WHERE site_id = $site", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "SELECT * FROM points WHERE site_id = $1");
        assert_eq!(out.params, vec![BoundParam::Text("Site-A".into())]);
    }

    #[test]
    fn brace_token_binds_single_value() {
        let vars = vec![one("site", "Site-A")];
        let out = lower("WHERE site_id = ${site}", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE site_id = $1");
        assert_eq!(out.params, vec![BoundParam::Text("Site-A".into())]);
    }

    #[test]
    fn csv_expands_each_value_as_its_own_param() {
        let vars = vec![many("site", &["A", "B", "C"])];
        let out = lower("WHERE site_id IN (${site:csv})", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE site_id IN ($1, $2, $3)");
        assert_eq!(out.params.len(), 3);
    }

    #[test]
    fn singlequote_expands_like_csv_but_each_is_bound() {
        let vars = vec![many("site", &["A", "B"])];
        let out = lower("WHERE site_id IN (${site:singlequote})", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE site_id IN ($1, $2)");
        assert_eq!(
            out.params,
            vec![BoundParam::Text("A".into()), BoundParam::Text("B".into())]
        );
    }

    #[test]
    fn sql_in_wraps_expansion_in_parens() {
        let vars = vec![many("site", &["A", "B"])];
        let out = lower("WHERE site_id $__sqlIn(site)", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE site_id IN ($1, $2)");
        assert_eq!(out.params.len(), 2);
    }

    #[test]
    fn start_index_offsets_placeholder_numbers() {
        // The datasource path already used $1; variables number from $2.
        let vars = vec![one("site", "A")];
        let out = lower("WHERE a = $1 AND b = $site", &vars, 1, None).unwrap();
        assert_eq!(out.sql, "WHERE a = $1 AND b = $2");
        assert_eq!(out.params, vec![BoundParam::Text("A".into())]);
    }

    #[test]
    fn existing_positional_placeholder_is_left_untouched() {
        let out = lower("WHERE a = $1", &[], 1, None).unwrap();
        assert_eq!(out.sql, "WHERE a = $1");
        assert!(out.params.is_empty());
    }

    #[test]
    fn injection_value_binds_as_literal_never_executes() {
        // The classic injection payload arrives as data and leaves as a single
        // bound parameter; the SQL text only gains a `$1` placeholder.
        let vars = vec![one("site", "'); DROP TABLE points; --")];
        let out = lower("WHERE site_id = $site", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE site_id = $1");
        assert_eq!(
            out.params,
            vec![BoundParam::Text("'); DROP TABLE points; --".into())]
        );
        // No SQL keyword from the payload is present in the rewritten text.
        assert!(!out.sql.contains("DROP"));
    }

    #[test]
    fn unknown_variable_is_an_error() {
        let err = lower("WHERE x = $missing", &[], 0, None).unwrap_err();
        assert_eq!(
            err,
            InterpolateError::UnknownVariable {
                name: "missing".into()
            }
        );
    }

    #[test]
    fn multi_value_in_single_token_is_an_error() {
        let vars = vec![many("site", &["A", "B"])];
        let err = lower("WHERE x = $site", &vars, 0, None).unwrap_err();
        assert_eq!(
            err,
            InterpolateError::MultiValueInSingle { name: "site".into() }
        );
    }

    #[test]
    fn empty_expansion_is_an_error() {
        let vars = vec![many("site", &[])];
        let err = lower("WHERE x IN (${site:csv})", &vars, 0, None).unwrap_err();
        assert_eq!(err, InterpolateError::EmptyExpansion { name: "site".into() });
    }

    #[test]
    fn unknown_format_is_an_error() {
        let vars = vec![one("site", "A")];
        let err = lower("WHERE x = ${site:nope}", &vars, 0, None).unwrap_err();
        assert_eq!(
            err,
            InterpolateError::UnknownFormat {
                name: "site".into(),
                format: "nope".into()
            }
        );
    }

    #[test]
    fn unterminated_brace_is_an_error() {
        let err = lower("WHERE x = ${site", &[], 0, None).unwrap_err();
        assert!(matches!(err, InterpolateError::Unterminated { .. }));
    }

    #[test]
    fn unterminated_sql_in_is_an_error() {
        let err = lower("WHERE x $__sqlIn(site", &[], 0, None).unwrap_err();
        assert!(matches!(err, InterpolateError::Unterminated { .. }));
    }

    #[test]
    fn lone_dollar_is_copied_verbatim() {
        let out = lower("SELECT '$' AS d", &[], 0, None).unwrap();
        assert_eq!(out.sql, "SELECT '$' AS d");
        assert!(out.params.is_empty());
    }

    #[test]
    fn no_variables_is_an_identity_rewrite() {
        let out = lower("SELECT 1", &[], 0, None).unwrap();
        assert_eq!(out.sql, "SELECT 1");
        assert!(out.params.is_empty());
    }

    #[test]
    fn mixed_tokens_number_in_order() {
        let vars = vec![one("a", "x"), many("b", &["y", "z"])];
        let out = lower("WHERE a = $a AND b IN (${b:csv})", &vars, 0, None).unwrap();
        assert_eq!(out.sql, "WHERE a = $1 AND b IN ($2, $3)");
        assert_eq!(out.params.len(), 3);
    }

    #[test]
    fn typed_scalars_map_to_bound_params() {
        let vars = vec![
            QueryVariable {
                name: "n".into(),
                value: VarValue::One(Scalar::Int(7)),
            },
            QueryVariable {
                name: "f".into(),
                value: VarValue::One(Scalar::Float(1.5)),
            },
            QueryVariable {
                name: "b".into(),
                value: VarValue::One(Scalar::Bool(true)),
            },
        ];
        let out = lower("WHERE n = $n AND f = $f AND b = $b", &vars, 0, None).unwrap();
        assert_eq!(
            out.params,
            vec![
                BoundParam::Int(7),
                BoundParam::Float(1.5),
                BoundParam::Bool(true)
            ]
        );
    }
}
