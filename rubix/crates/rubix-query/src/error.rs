//! The query-crate error domain.
//!
//! `rubix-query` sits above SurrealDB only for unification and heavy vectorized
//! aggregation (`rubix/STACK-DEISGN.md`, contract #6). Its failures are distinct
//! from the store/gate domains: a statement the read-only guard rejects, a
//! capability the gate denies, a DataFusion planning/execution failure, and a
//! SurrealDB read failure when scanning a canonical table. Each converts into the
//! project [`Error`](rubix_core::Error) at the crate boundary so callers chain
//! with `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error;

/// Convenience alias for the query-crate result.
pub type Result<T> = std::result::Result<T, QueryError>;

/// A failure raised while planning or running a query, rollup, or vector search.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    /// The submitted SQL was not a single read-only statement.
    ///
    /// The unified surface accepts `SELECT`/`WITH` only; anything else (a write,
    /// a DDL statement, or multiple statements) is refused before execution
    /// (`rubix/docs/sessions/WS-09.md`, statement guard).
    #[error("query rejected: {0}")]
    Rejected(String),

    /// The principal lacks the capability the query action requires.
    ///
    /// The query action is app-enforced (contract #2): a principal without the
    /// grant is denied before any scan runs.
    #[error("query denied: principal lacks the query capability")]
    Denied,

    /// Scanning a canonical table through the scoped session failed.
    #[error("scan error: {0}")]
    Scan(String),

    /// DataFusion failed to plan or execute the query.
    #[error("datafusion error: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),

    /// The gate's capability check itself failed (surfaced, never read as allow).
    #[error("capability check error: {0}")]
    Capability(String),
}

impl From<QueryError> for Error {
    fn from(error: QueryError) -> Self {
        Error::Store(error.to_string())
    }
}
