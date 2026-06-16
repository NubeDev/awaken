//! Expand the backend time macros in a chart's SQL.
//!
//! The board path no longer splices a locale datetime string into SQL
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §5 — the timezone bug). Instead a
//! chart authors **macros** that the backend rewrites into real SQL against the
//! resolved UTC window and the snapped [`Grain`], so the window math and the
//! interval snap live in exactly one place (the backend) and a chart and a rule
//! bucket line up. The rewrite is a pure string substitution that runs **before**
//! the read-only guard, so the guard still sees — and vets — the final statement.
//!
//! The macros (Grafana-shaped, deliberately small):
//! - `$__timeFilter(<col>)` → `<col> BETWEEN <from> AND <to>` (UTC bounds).
//! - `$__timeBucket(<col>)` → an epoch-aligned floor of `<col>` to the resolved
//!   grain, returned as a timestamp (reuses [`Grain::bucket_start`]'s alignment).
//! - `$__interval` → the resolved grain as a quoted string literal.
//!
//! `$__timeBucket` and `$__interval` require a grain on the [`TimeScope`]; using
//! them with no grain is a rejected statement rather than a silent passthrough.

use crate::aggregate::Grain;
use crate::error::{QueryError, Result};

use super::scope::ResolvedTimeScope;

/// The UTC-bounds filter macro.
const FILTER_MACRO: &str = "$__timeFilter(";
/// The epoch-aligned bucket macro.
const BUCKET_MACRO: &str = "$__timeBucket(";
/// The resolved-grain literal macro.
const INTERVAL_MACRO: &str = "$__interval";

/// Rewrite every time macro in `sql` against `scope`.
///
/// Returns the SQL unchanged if it carries no macros, so a non-time chart is
/// unaffected. The `$__timeFilter`/`$__timeBucket` calls take a single column
/// expression argument (typically `created`); the argument is spliced verbatim,
/// so it must be a bare column or expression the surrounding query already trusts.
///
/// # Errors
/// Returns [`QueryError::Rejected`] if a bucket/interval macro is used without a
/// grain on the scope, or if a macro call is malformed (an unclosed paren).
pub fn expand_macros(sql: &str, scope: &ResolvedTimeScope) -> Result<String> {
    let mut out = expand_call(sql, FILTER_MACRO, &|col| filter_sql(col, scope))?;
    out = expand_call(&out, BUCKET_MACRO, &|col| bucket_sql(col, scope))?;
    if out.contains(INTERVAL_MACRO) {
        out = out.replace(INTERVAL_MACRO, &interval_sql(scope)?);
    }
    Ok(out)
}

/// Expand every `name(<arg>)` macro call in `sql`, replacing each with the result
/// of `render` applied to the argument text between the parentheses.
fn expand_call(
    sql: &str,
    name: &str,
    render: &dyn Fn(&str) -> Result<String>,
) -> Result<String> {
    let mut out = String::with_capacity(sql.len());
    let mut rest = sql;
    while let Some(at) = rest.find(name) {
        out.push_str(&rest[..at]);
        let after = &rest[at + name.len()..];
        let close = after
            .find(')')
            .ok_or_else(|| QueryError::Rejected(format!("unclosed {name}…) macro")))?;
        let arg = after[..close].trim();
        out.push_str(&render(arg)?);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// `<col> BETWEEN <from> AND <to>` against the scope's UTC bounds.
fn filter_sql(col: &str, scope: &ResolvedTimeScope) -> Result<String> {
    Ok(format!(
        "{col} BETWEEN {} AND {}",
        timestamp_literal(scope.from_micros()),
        timestamp_literal(scope.to_micros()),
    ))
}

/// An epoch-aligned floor of `<col>` to the resolved grain, as a timestamp.
///
/// Casting the microsecond timestamp to `BIGINT` yields epoch micros; flooring to
/// a multiple of the grain width and casting back reproduces
/// [`Grain::bucket_start`] in SQL, so a chart bucket and a rule bucket coincide.
fn bucket_sql(col: &str, scope: &ResolvedTimeScope) -> Result<String> {
    let width = require_grain(scope)?.width_micros();
    Ok(format!(
        "arrow_cast((CAST({col} AS BIGINT) / {width}) * {width}, 'Timestamp(Microsecond, None)')"
    ))
}

/// The resolved grain as a quoted SQL string literal.
fn interval_sql(scope: &ResolvedTimeScope) -> Result<String> {
    Ok(format!("'{}'", require_grain(scope)?.as_str()))
}

/// The scope's grain, or a rejection if a bucket/interval macro needs one.
fn require_grain(scope: &ResolvedTimeScope) -> Result<Grain> {
    scope.grain().ok_or_else(|| {
        QueryError::Rejected(
            "a time bucket/interval macro requires a grain or target_points on the time scope"
                .to_owned(),
        )
    })
}

/// A DataFusion timestamp literal for `micros` epoch microseconds.
///
/// `arrow_cast` of the integer epoch micros to a microsecond timestamp is exact
/// and timezone-free (the canonical `created` column is `Timestamp(us, None)`), so
/// the comparison is a pure UTC instant comparison — no parsing, no locale.
fn timestamp_literal(micros: i64) -> String {
    format!("arrow_cast({micros}, 'Timestamp(Microsecond, None)')")
}

#[cfg(test)]
mod tests {
    use super::expand_macros;
    use crate::aggregate::Grain;
    use crate::time::scope::ResolvedTimeScope;

    fn scope_with_grain() -> ResolvedTimeScope {
        ResolvedTimeScope::new(1_000_000, 2_000_000, Some(Grain::Hour))
    }

    fn scope_no_grain() -> ResolvedTimeScope {
        ResolvedTimeScope::new(1_000_000, 2_000_000, None)
    }

    #[test]
    fn filter_macro_expands_to_a_between_on_utc_bounds() {
        let out = expand_macros("SELECT * FROM record WHERE $__timeFilter(created)", &scope_no_grain())
            .unwrap();
        assert!(out.contains("created BETWEEN arrow_cast(1000000"), "{out}");
        assert!(out.contains("arrow_cast(2000000"), "{out}");
        assert!(!out.contains("$__"), "macro fully expanded: {out}");
    }

    #[test]
    fn bucket_macro_floors_to_the_grain_width() {
        let out = expand_macros("SELECT $__timeBucket(created) AS t FROM record", &scope_with_grain())
            .unwrap();
        let width = Grain::Hour.width_micros();
        assert!(out.contains(&format!("/ {width}) * {width}")), "{out}");
        assert!(out.contains("CAST(created AS BIGINT)"), "{out}");
    }

    #[test]
    fn interval_macro_becomes_the_grain_literal() {
        let out = expand_macros("SELECT $__interval", &scope_with_grain()).unwrap();
        assert_eq!(out, "SELECT 'hour'");
    }

    #[test]
    fn a_chart_with_no_macros_is_unchanged() {
        let sql = "SELECT count(*) FROM record";
        assert_eq!(expand_macros(sql, &scope_with_grain()).unwrap(), sql);
    }

    #[test]
    fn bucket_without_a_grain_is_rejected() {
        let err = expand_macros("SELECT $__timeBucket(created)", &scope_no_grain()).unwrap_err();
        assert!(err.to_string().contains("grain"), "{err}");
    }

    #[test]
    fn an_unclosed_macro_is_rejected() {
        assert!(expand_macros("SELECT $__timeFilter(created", &scope_no_grain()).is_err());
    }

    #[test]
    fn multiple_filter_calls_all_expand() {
        let out = expand_macros(
            "SELECT * FROM record WHERE $__timeFilter(created) OR $__timeFilter(updated)",
            &scope_no_grain(),
        )
        .unwrap();
        assert!(out.contains("created BETWEEN"), "{out}");
        assert!(out.contains("updated BETWEEN"), "{out}");
    }
}
