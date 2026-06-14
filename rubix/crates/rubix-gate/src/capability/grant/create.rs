//! Persist a capability grant — in-scope, no privilege escalation.
//!
//! Creating a grant is a privileged action: the grantor must hold the authority
//! to confer it (`rubix/docs/SCOPE.md`, "Capabilities are grants"). This verb
//! checks [`may_administer`] *before* any write, so an unauthorized grantor is
//! refused without touching the store. A grant for a known capability is written
//! to a deterministic key, so creating the same grant twice is idempotent.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::capability::kind::Capability;
use crate::error::{GateError, Result};

use super::authority::may_administer;
use super::model::Grant;
use super::row::{GRANT_TABLE, GrantRow, grant_key};

/// Grant `capability` to `grantee`, authorized by `grantor`.
///
/// The grant is bound to the grantee's subject and namespace. `grantor` must be
/// an admin in the grantee's namespace; otherwise the grant is refused before
/// any write (fail closed, no escalation).
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `grantor` lacks the authority to
/// confer the grant, or [`GateError::GrantStore`] if the write fails.
pub async fn create_grant(
    db: &Surreal<Db>,
    grantor: &Principal,
    grantee: &Principal,
    capability: Capability,
) -> Result<Grant> {
    let grant = Grant::new(grantee, capability);
    if !may_administer(grantor, &grant) {
        return Err(GateError::GrantDenied(format!(
            "{} may not grant {} in namespace {}",
            grantor.subject,
            capability.as_str(),
            grant.namespace
        )));
    }
    let row = GrantRow::from_grant(&grant);
    let _: Option<GrantRow> = db
        .upsert((GRANT_TABLE, grant_key(&grant)))
        .content(row)
        .await
        .map_err(GateError::GrantStore)?;
    Ok(grant)
}
