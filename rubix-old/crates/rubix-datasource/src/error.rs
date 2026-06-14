//! The one domain error for reading external datasources.
//!
//! A single `thiserror` enum covers the whole crate so callers match on one
//! type. Variants carry stringified causes (rather than `#[from]` on the sqlx
//! error) so a connection or backend message can never carry a decrypted
//! password back to a caller or a log — the registry is the only place that
//! sees credentials (see [`crate::registry`]).

use thiserror::Error;

/// Failure resolving, describing, or reading a datasource.
#[derive(Debug, Error)]
pub enum DatasourceError {
    /// No datasource is registered under the requested id.
    #[error("unknown datasource `{0}`")]
    UnknownDatasource(String),

    /// The named query is not registered on this datasource.
    #[error("unknown named query `{query}` on datasource `{datasource}`")]
    UnknownQuery { datasource: String, query: String },

    /// A named-query invocation supplied the wrong number of parameters.
    #[error("named query `{query}` expects {expected} parameter(s), got {got}")]
    ParamCount {
        query: String,
        expected: usize,
        got: usize,
    },

    /// The SQL text carried more than one statement (rejected before execution
    /// so a `SELECT`-only role cannot be bypassed by a trailing statement).
    #[error("multi-statement SQL is rejected: exactly one statement per call")]
    MultiStatement,

    /// The SQL text was empty after trimming.
    #[error("empty SQL statement")]
    EmptyStatement,

    /// The connection pool could not be opened for a datasource.
    #[error("connect datasource `{datasource}`: {message}")]
    Connect { datasource: String, message: String },

    /// A read against the external engine failed (planning, execution, decode).
    #[error("backend read on `{datasource}`: {message}")]
    Backend { datasource: String, message: String },

    /// The result breached a cap and the caller chose the strict (error) path.
    /// Carries what was collected before the breach so the caller can report it.
    #[error("result cap breached on `{datasource}`: {rows} row(s), {bytes} byte(s)")]
    CapBreached {
        datasource: String,
        rows: u64,
        bytes: u64,
    },

    /// The manifest could not be parsed.
    #[error("parse datasource manifest: {0}")]
    Manifest(#[from] serde_json::Error),
}

/// Crate-wide result alias.
pub type DatasourceResult<T> = Result<T, DatasourceError>;
