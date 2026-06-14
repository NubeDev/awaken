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

    /// A grantor lacked the authority to create or revoke a grant (the
    /// app-enforced capability layer failing closed, `rubix/docs/SCOPE.md`).
    #[error("not authorized to administer grant: {0}")]
    GrantDenied(String),

    /// Persisting, listing, or revoking a capability grant failed.
    #[error("grant store operation failed: {0}")]
    GrantStore(#[source] surrealdb::Error),

    /// A command's principal lacked the capability grant required to apply it.
    /// The command is refused before any write occurs (`rubix/docs/SCOPE.md`,
    /// "Commands go through the gate").
    #[error("command denied: {0}")]
    CommandDenied(String),

    /// Applying a command's record mutation through the gate failed, or the
    /// atomic before/after capture could not be decoded.
    #[error("command apply failed: {0}")]
    CommandApply(#[source] surrealdb::Error),

    /// Writing the append-only audit row for an applied command failed.
    #[error("audit write failed: {0}")]
    AuditWrite(#[source] surrealdb::Error),

    /// A change was refused at the undo boundary: undo covers user-facing
    /// definitions only — never the data plane (readings, insight firings) and
    /// never the audit log (`rubix/docs/SCOPE.md`, "Undo/redo").
    #[error("undo boundary: {0}")]
    UndoBoundary(String),

    /// There was nothing left to undo (or redo) for the principal + resource —
    /// the stack for that slot is empty.
    #[error("nothing to reverse: {0}")]
    NothingToReverse(String),
}

impl From<GateError> for CoreError {
    fn from(err: GateError) -> Self {
        CoreError::Store(err.to_string())
    }
}
