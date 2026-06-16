//! Inject dashboard variables into a chart's SQL — the injection boundary.
//!
//! A templated chart authors variable references (`$site`, `${site:csv}`,
//! `$__sqlIn(site)`) that a dashboard's variable bar fills in, so **one** board
//! serves a whole fleet instead of one hand-authored board per site
//! (`rubix/docs/design/variables-and-templating.md`). rubix has no SQL bind layer
//! on the unified surface — `ctx.sql(sql)` takes a string only — so this engine
//! **owns the quoting**: every value is lowered into a complete, escaped SQL
//! literal, server-side, and the read-only guard then vets the final statement.
//! That is the whole security story (and why an author must never wrap a variable
//! in their own quotes — the engine emits the quotes):
//!
//! - A `Text` value becomes a single-quoted literal with every `'` doubled — the
//!   standard-SQL escape DataFusion honours — so `'); DROP TABLE record; --` binds
//!   as the literal string `'''); DROP TABLE record; --'` and cannot break out.
//! - A `Num`/`Bool` value is a closed character set (digits, sign, `.`, `e`, or
//!   `TRUE`/`FALSE`) lowered bare; it can carry no metacharacter.
//!
//! Like the time-macro rewrite this is a pure string substitution that runs
//! **before** [`ensure_read_only`](crate::ensure_read_only), so even if a value
//! somehow closed its literal the guard still rejects a second statement.
//!
//! Reference forms (lowered after the time macros, which own the `$__time*`
//! tokens):
//! - `$name` / `${name}` → the variable's first value as one literal (`NULL` if
//!   the selection is empty).
//! - `${name:csv}` → every value as a typed literal, comma-joined (for an explicit
//!   `IN (...)` list); empty → `NULL`.
//! - `${name:singlequote}` → every value force-quoted as a string, comma-joined;
//!   empty → `NULL`.
//! - `$__sqlIn(name)` → a parenthesised `(v1, v2, …)` list ready for `IN`; an empty
//!   selection becomes `(NULL)` so the predicate matches nothing rather than
//!   failing to parse.
//!
//! A bare `$name` whose variable was not supplied is left untouched (so a literal
//! `$` or an unrelated token survives); the explicit `${…}` / `$__sqlIn(…)` forms
//! name an intent, so an unknown variable there is a rejection.

use crate::error::{QueryError, Result};

mod value;

pub use value::Scalar;

/// One resolved dashboard variable: its name and selected value(s).
///
/// Zero values is an empty selection (lowers to `NULL`); one value is the common
/// single-select; many values is a multi-select feeding `csv`/`singlequote`/
/// `$__sqlIn`. Construction validates the values are scalars — see
/// [`Scalar::from_json`].
#[derive(Debug, Clone)]
pub struct Variable {
    /// The reference name, without the leading `$` (`site`, not `$site`).
    pub name: String,
    /// The selected value(s); empty is a valid empty selection.
    pub values: Vec<Scalar>,
}

impl Variable {
    /// A variable with a single value.
    #[must_use]
    pub fn single(name: impl Into<String>, value: Scalar) -> Self {
        Self {
            name: name.into(),
            values: vec![value],
        }
    }
}

/// Lower every variable reference in `sql` against `variables`.
///
/// Returns the SQL unchanged when it carries no resolvable reference, so a
/// non-templated chart is unaffected and the engine stays opt-in. The result is
/// the statement the engine runs; it still passes through the read-only guard at
/// the call site.
///
/// # Errors
/// Returns [`QueryError::Rejected`] if an explicit `${…}` / `$__sqlIn(…)` form
/// names an unsupplied variable, carries an unknown modifier, or is malformed (an
/// unclosed brace/paren).
pub fn expand_variables(sql: &str, variables: &[Variable]) -> Result<String> {
    if !sql.contains('$') {
        return Ok(sql.to_owned());
    }
    let mut out = String::with_capacity(sql.len());
    let mut rest = sql;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        match lower_reference(after, variables)? {
            Some((rendered, consumed)) => {
                out.push_str(&rendered);
                rest = &after[consumed..];
            }
            None => {
                // Not a variable reference (a bare `$`, a `$1` placeholder, or an
                // unsupplied bare `$name`): keep the `$` and carry on.
                out.push('$');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Try to lower one reference that begins immediately after a `$`.
///
/// `after` is the slice following the `$`. Returns `Some((rendered, consumed))`
/// where `consumed` counts bytes of `after` that the reference spanned, or `None`
/// when the text is not a reference we own (the caller then emits a literal `$`).
fn lower_reference(after: &str, variables: &[Variable]) -> Result<Option<(String, usize)>> {
    if let Some(body) = after.strip_prefix('{') {
        let close = body
            .find('}')
            .ok_or_else(|| QueryError::Rejected("unclosed ${…} variable".to_owned()))?;
        let (name, modifier) = split_modifier(body[..close].trim());
        let variable = require(variables, name)?;
        return Ok(Some((render_modified(variable, modifier)?, 1 + close + 1)));
    }
    if let Some(body) = after.strip_prefix("__sqlIn(") {
        let close = body
            .find(')')
            .ok_or_else(|| QueryError::Rejected("unclosed $__sqlIn(…) macro".to_owned()))?;
        let name = body[..close].trim();
        let variable = require(variables, name)?;
        return Ok(Some((render_sql_in(variable), "__sqlIn(".len() + close + 1)));
    }
    // Bare `$name`: a letter start (so `$__…` builtins are never swallowed) then
    // word characters. An unsupplied bare name is left as a literal `$`.
    let len = bare_name_len(after);
    if len == 0 {
        return Ok(None);
    }
    let name = &after[..len];
    match find(variables, name) {
        Some(variable) => Ok(Some((render_single(variable), len))),
        None => Ok(None),
    }
}

/// Split `body` of a `${…}` into `(name, optional modifier)` on the first `:`.
fn split_modifier(body: &str) -> (&str, Option<&str>) {
    match body.split_once(':') {
        Some((name, modifier)) => (name.trim(), Some(modifier.trim())),
        None => (body, None),
    }
}

/// The byte length of a bare `$name` at the start of `after`, or `0` if none.
///
/// Requires a leading ASCII letter so a leading-underscore builtin (`$__interval`)
/// is never read as a variable; continues over `[A-Za-z0-9_]`.
fn bare_name_len(after: &str) -> usize {
    let bytes = after.as_bytes();
    if bytes.first().is_none_or(|b| !b.is_ascii_alphabetic()) {
        return 0;
    }
    bytes
        .iter()
        .position(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
        .unwrap_or(bytes.len())
}

/// Find a supplied variable by name.
fn find<'a>(variables: &'a [Variable], name: &str) -> Option<&'a Variable> {
    variables.iter().find(|v| v.name == name)
}

/// Find a variable an explicit reference named, or reject — the explicit forms
/// declare an intent, so a missing variable is an error rather than a passthrough.
fn require<'a>(variables: &'a [Variable], name: &str) -> Result<&'a Variable> {
    find(variables, name)
        .ok_or_else(|| QueryError::Rejected(format!("unknown dashboard variable: ${name}")))
}

/// The first value as one literal, or `NULL` for an empty selection.
fn render_single(variable: &Variable) -> String {
    variable
        .values
        .first()
        .map_or_else(|| "NULL".to_owned(), Scalar::to_literal)
}

/// Apply a `${name:modifier}` modifier (or none) to a variable.
fn render_modified(variable: &Variable, modifier: Option<&str>) -> Result<String> {
    match modifier {
        None => Ok(render_single(variable)),
        Some("csv") => Ok(join_or_null(variable, Scalar::to_literal)),
        Some("singlequote") => Ok(join_or_null(variable, Scalar::to_quoted)),
        Some(other) => Err(QueryError::Rejected(format!(
            "unknown variable modifier: {other}"
        ))),
    }
}

/// `(v1, v2, …)` for an `IN` list; an empty selection is `(NULL)` (matches none).
fn render_sql_in(variable: &Variable) -> String {
    if variable.values.is_empty() {
        return "(NULL)".to_owned();
    }
    format!("({})", join(variable, Scalar::to_literal))
}

/// Join the values with `render`, or `NULL` when the selection is empty.
fn join_or_null(variable: &Variable, render: fn(&Scalar) -> String) -> String {
    if variable.values.is_empty() {
        "NULL".to_owned()
    } else {
        join(variable, render)
    }
}

/// Comma-join the rendered values.
fn join(variable: &Variable, render: fn(&Scalar) -> String) -> String {
    variable
        .values
        .iter()
        .map(render)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{Scalar, Variable, expand_variables};

    fn text(name: &str, values: &[&str]) -> Variable {
        Variable {
            name: name.to_owned(),
            values: values.iter().map(|v| Scalar::Text((*v).to_owned())).collect(),
        }
    }

    #[test]
    fn a_bare_variable_becomes_a_quoted_literal() {
        let out = expand_variables("SELECT * FROM record WHERE site = $site", &[text("site", &["hq"])])
            .unwrap();
        assert_eq!(out, "SELECT * FROM record WHERE site = 'hq'");
    }

    #[test]
    fn the_brace_form_is_equivalent_to_the_bare_form() {
        let vars = [text("site", &["hq"])];
        assert_eq!(
            expand_variables("WHERE site = ${site}", &vars).unwrap(),
            "WHERE site = 'hq'"
        );
    }

    #[test]
    fn an_injection_payload_binds_as_a_literal() {
        let out = expand_variables(
            "WHERE site = $site",
            &[text("site", &["'); DROP TABLE record; --"])],
        )
        .unwrap();
        assert_eq!(out, "WHERE site = '''); DROP TABLE record; --'");
        // The single quote that would close the literal is doubled, so the payload
        // stays inside one string literal and cannot start a second statement.
    }

    #[test]
    fn an_apostrophe_value_is_doubled() {
        let out = expand_variables("WHERE name = $name", &[text("name", &["o'brien"])]).unwrap();
        assert_eq!(out, "WHERE name = 'o''brien'");
    }

    #[test]
    fn csv_joins_typed_literals() {
        let out = expand_variables(
            "WHERE site IN (${site:csv})",
            &[text("site", &["hq", "tower"])],
        )
        .unwrap();
        assert_eq!(out, "WHERE site IN ('hq', 'tower')");
    }

    #[test]
    fn sql_in_wraps_the_list_in_parens() {
        let out = expand_variables(
            "WHERE site IN $__sqlIn(site)",
            &[text("site", &["hq", "tower"])],
        )
        .unwrap();
        assert_eq!(out, "WHERE site IN ('hq', 'tower')");
    }

    #[test]
    fn an_empty_selection_lowers_to_null_forms() {
        let vars = [text("site", &[])];
        assert_eq!(expand_variables("= $site", &vars).unwrap(), "= NULL");
        assert_eq!(expand_variables("(${site:csv})", &vars).unwrap(), "(NULL)");
        assert_eq!(
            expand_variables("IN $__sqlIn(site)", &vars).unwrap(),
            "IN (NULL)"
        );
    }

    #[test]
    fn numbers_and_bools_lower_bare_quoted_only_under_singlequote() {
        let vars = [Variable {
            name: "n".to_owned(),
            values: vec![Scalar::Num("42".to_owned()), Scalar::Bool(true)],
        }];
        assert_eq!(expand_variables("x = $n", &vars).unwrap(), "x = 42");
        assert_eq!(
            expand_variables("(${n:csv})", &vars).unwrap(),
            "(42, TRUE)"
        );
        assert_eq!(
            expand_variables("(${n:singlequote})", &vars).unwrap(),
            "('42', 'true')"
        );
    }

    #[test]
    fn an_unsupplied_bare_name_is_left_as_a_literal_dollar() {
        let out = expand_variables("WHERE cost > $5 AND x = $site", &[text("site", &["hq"])]).unwrap();
        assert_eq!(out, "WHERE cost > $5 AND x = 'hq'");
    }

    #[test]
    fn a_builtin_token_is_not_swallowed_as_a_variable() {
        // `$__interval` would already be expanded by the time pass; here we prove
        // the variable pass never mistakes a leading-underscore token for a var.
        let out = expand_variables("SELECT $__interval, $site", &[text("site", &["hq"])]).unwrap();
        assert_eq!(out, "SELECT $__interval, 'hq'");
    }

    #[test]
    fn an_explicit_unknown_variable_is_rejected() {
        assert!(expand_variables("${ghost}", &[]).is_err());
        assert!(expand_variables("$__sqlIn(ghost)", &[]).is_err());
    }

    #[test]
    fn an_unknown_modifier_is_rejected() {
        assert!(expand_variables("${site:bogus}", &[text("site", &["hq"])]).is_err());
    }

    #[test]
    fn an_unclosed_form_is_rejected() {
        assert!(expand_variables("${site", &[text("site", &["hq"])]).is_err());
        assert!(expand_variables("$__sqlIn(site", &[text("site", &["hq"])]).is_err());
    }

    #[test]
    fn a_chart_with_no_dollar_is_unchanged() {
        let sql = "SELECT count(*) FROM record";
        assert_eq!(expand_variables(sql, &[text("site", &["hq"])]).unwrap(), sql);
    }

    #[test]
    fn adjacent_names_do_not_collide_on_a_prefix() {
        // `$site` must not greedily match inside `$site_id`.
        let vars = [text("site", &["a"]), text("site_id", &["b"])];
        let out = expand_variables("$site_id = $site", &vars).unwrap();
        assert_eq!(out, "'b' = 'a'");
    }
}
