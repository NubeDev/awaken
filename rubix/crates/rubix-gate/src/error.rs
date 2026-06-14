//! Gate-domain errors, converted into the project error at the crate boundary.
//!
//! The gate's failures are authentication and scoping failures; record-read
//! failures from the scoped session surface through the store boundary. These
//! convert into the project [`Error`](rubix_core::Error) so callers chain with
//! `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error as CoreError;

/// Convenience alias for gate results.
pub type Result<T> = std::result::Result<T, GateError>;

/// Failures raised by the access gate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GateError {
    /// A token did not resolve to a principal.
    #[error("authentication failed: {0}")]
    Authenticate(String),

    /// Minting a scoped SurrealDB session for a principal failed.
    #[error("failed to issue scoped session: {0}")]
    IssueSession(#[source] surrealdb::Error),

    /// Defining the principal access method or record permissions failed.
    #[error("failed to define gate schema: {0}")]
    DefineSchema(#[source] surrealdb::Error),

    /// A direct SurrealDB lookup on the root handle failed (e.g. resolving a
    /// principal during authentication).
    #[error("principal lookup failed: {0}")]
    Lookup(#[source] surrealdb::Error),

    /// A scoped read against the principal's session failed.
    #[error("scoped read failed: {0}")]
    Read(#[source] CoreError),
}

impl From<GateError> for CoreError {
    fn from(err: GateError) -> Self {
        CoreError::Store(err.to_string())
    }
}
