//! Interpolation errors. A token the engine cannot resolve or bind is a clear
//! error the caller can correct — never a silent passthrough that would leave a
//! raw `$token` in the SQL (docs/design/variables-and-templating.md §2).

use thiserror::Error;

/// A failure lowering variable tokens into bound parameters.
#[derive(Debug, Error, PartialEq)]
pub enum InterpolateError {
    /// A `$name` / `${name}` / `$__sqlIn(name)` token referenced a variable the
    /// caller did not supply. Refused rather than left in the SQL.
    #[error("unknown variable `${name}` referenced in SQL")]
    UnknownVariable {
        /// The variable name the token referenced.
        name: String,
    },

    /// A single-value token (`$name` / `${name}`) referenced a variable carrying
    /// multiple values. The caller must use a multi token (`:csv`,
    /// `:singlequote`, or `$__sqlIn`) for a multi-value variable.
    #[error("variable `${name}` has multiple values; use ${{{name}:csv}}, ${{{name}:singlequote}}, or $__sqlIn({name})")]
    MultiValueInSingle {
        /// The variable name the single-value token referenced.
        name: String,
    },

    /// A multi-expansion token (`:csv`, `:singlequote`, `$__sqlIn`) referenced a
    /// variable with no values, which cannot form a parameter list.
    #[error("variable `${name}` has no values to expand")]
    EmptyExpansion {
        /// The variable name the expansion token referenced.
        name: String,
    },

    /// A `${...}` token carried a format suffix the engine does not implement.
    #[error("unknown variable format `${{{name}:{format}}}`")]
    UnknownFormat {
        /// The variable name.
        name: String,
        /// The unrecognised format suffix.
        format: String,
    },

    /// A `${...}` brace token or `$__sqlIn(...)` call was not closed.
    #[error("unterminated variable token in SQL near `{near}`")]
    Unterminated {
        /// A short prefix of the offending token for the caller to locate it.
        near: String,
    },

    /// A time-range bound was neither an RFC 3339 instant nor a recognised
    /// relative token (`now`, `now-6h`, `now/d`). Refused rather than reaching
    /// SQL (docs/design/time-range-and-refresh.md §1).
    #[error("invalid time-range token `{token}`")]
    BadTimeToken {
        /// The offending bound token.
        token: String,
    },

    /// A resolved time range was empty (`from >= to`), which no time macro can
    /// satisfy. Refused so a degenerate range surfaces as an error.
    #[error("empty time range: `from` ({from}) is not before `to` ({to})")]
    EmptyRange {
        /// The unresolved lower-bound token.
        from: String,
        /// The unresolved upper-bound token.
        to: String,
    },

    /// A time macro (`$__from`, `$__timeFilter`, …) appeared in SQL but the
    /// request carried no `time_range`. Refused rather than leaving the macro
    /// unbound (docs/design/time-range-and-refresh.md §4).
    #[error("time macro `{macro_name}` used but no time range was supplied")]
    MissingTimeRange {
        /// The macro that needed a time range.
        macro_name: String,
    },

    /// A `$__timeFilter(col)` / `$__timeGroup(col, ...)` call was malformed
    /// (missing column, missing `)` , or — for `$__timeGroup` — missing the
    /// interval argument).
    #[error("malformed time macro near `{near}`")]
    MalformedTimeMacro {
        /// A short prefix of the offending macro for the caller to locate it.
        near: String,
    },
}
