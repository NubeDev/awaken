//! Revoke a capability grant — same authority rule as creating one.
//!
//! Revoking is administered with the same fail-closed authority check as
//! granting (`rubix/docs/SCOPE.md`, "applied through the gate"): the grantor
//! must be an admin in the grantee's namespace. Revoking a grant that does not
//! exist is a no-op, so revoke is idempotent.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::capability::kind::Capability;
use crate::error::{GateError, Result};

use super::authority::may_administer;
use super::model::Grant;
use super::row::{GRANT_TABLE, GrantRow, grant_key};

/// Revoke `capability` from `grantee`, authorized by `grantor`.
///
/// `grantor` must be an admin in the grantee's namespace; otherwise the request
/// is refused before any write. Revoking an absent grant succeeds (idempotent).
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `grantor` lacks the authority, or
/// [`GateError::GrantStore`] if the delete fails.
pub async fn revoke_grant(
    db: &Surreal<Db>,
    grantor: &Principal,
    grantee: &Principal,
    capability: Capability,
) -> Result<()> {
    let grant = Grant::new(grantee, capability);
    if !may_administer(grantor, &grant) {
        return Err(GateError::GrantDenied(format!(
            "{} may not revoke {} in namespace {}",
            grantor.subject,
            capability.as_str(),
            grant.namespace
        )));
    }
    let _: Option<GrantRow> = db
        .delete((GRANT_TABLE, grant_key(&grant)))
        .await
        .map_err(GateError::GrantStore)?;
    Ok(())
}
