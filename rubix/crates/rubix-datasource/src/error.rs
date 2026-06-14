//! The datasource-crate error domain.
//!
//! `rubix-datasource` is the pluggable-connector layer over the DataFusion query
//! surface (`rubix/docs/SCOPE.md`, "Datasources"). Its failures are distinct from
//! the query/gate domains: a register denied because the principal lacks the
//! `datasource-register` capability, a lookup for a datasource id no connector was
//! registered under, a connector that could not build its `TableProvider`, and a
//! capability check that itself failed (surfaced, never read as allow). Each
//! converts into the project [`Error`](rubix_core::Error) at the crate boundary so
//! callers chain with `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error;

/// Convenience alias for the datasource-crate result.
pub type Result<T> = std::result::Result<T, DatasourceError>;

/// A failure raised while registering, resolving, or building a datasource.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DatasourceError {
    /// Registering a connector was denied: the principal lacks the
    /// `datasource-register` capability (contract #2, fail closed).
    #[error("datasource register denied: principal lacks the datasource-register capability")]
    Denied,

    /// The gate's capability check itself failed (surfaced, never read as allow).
    #[error("capability check error: {0}")]
    Capability(String),

    /// No connector is registered under the requested datasource id.
    #[error("unknown datasource: no connector registered under id `{0}`")]
    Unknown(String),

    /// A connector with the same id is already registered.
    ///
    /// Registration is explicit and idempotent only by intent: re-registering an
    /// id is refused rather than silently overwriting a live datasource.
    #[error("duplicate datasource: a connector is already registered under id `{0}`")]
    Duplicate(String),

    /// The connector could not build its DataFusion `TableProvider`.
    #[error("connector `{id}` failed to build a table provider: {reason}")]
    Connect {
        /// The datasource id whose connector failed.
        id: String,
        /// The underlying connector error message.
        reason: String,
    },

    /// DataFusion failed to register a provider or run the spanning query.
    #[error("datafusion error: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),

    /// The underlying query surface (`rubix-query`) failed.
    #[error("query error: {0}")]
    Query(String),
}

impl From<rubix_query::QueryError> for DatasourceError {
    fn from(error: rubix_query::QueryError) -> Self {
        DatasourceError::Query(error.to_string())
    }
}

impl From<DatasourceError> for Error {
    fn from(error: DatasourceError) -> Self {
        Error::Store(error.to_string())
    }
}
