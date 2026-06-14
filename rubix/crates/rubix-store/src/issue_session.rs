//! Scoped-session issuance seam.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): reads run on a gate-issued **scoped
//! SurrealDB session** enforced by SurrealDB row-level permissions, never
//! proxied per message. The real implementation — minting a scoped session
//! token for a principal — lands in WS-03 (identity + scoped read session).
//!
//! This seam fixes the boundary: WS-03 fills [`issue_scoped_session`] without the
//! store handle's other verbs changing shape.

use crate::error::Result;
use crate::handle::StoreHandle;

/// A scoped read session bound to a principal's permissions.
///
/// Today this carries only the handle it was issued from. WS-03 extends it with
/// the principal identity and the SurrealDB row-level scope.
#[derive(Clone)]
pub struct ScopedSession {
    handle: StoreHandle,
}

impl ScopedSession {
    /// The store handle this session reads through.
    #[must_use]
    pub fn handle(&self) -> &StoreHandle {
        &self.handle
    }
}

/// Issue a scoped read session from the store handle.
///
/// WS-03 replaces the body with real scoping (principal-bound SurrealDB session
/// plus row-level permissions). Until then it yields an unscoped session over
/// the same handle so dependent wiring can compile against the final signature.
///
/// # Errors
/// Will return a [`StoreError`](crate::error::StoreError) once WS-03 performs a
/// fallible scoping step; the seam keeps the `Result` signature stable.
pub fn issue_scoped_session(handle: &StoreHandle) -> Result<ScopedSession> {
    Ok(ScopedSession {
        handle: handle.clone(),
    })
}
