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
    write_grant(db, grantor, Grant::new(grantee, capability)).await
}

/// Grant `capability` to the **team** `slug` in `namespace`, authorized by
/// `grantor`.
///
/// The grant is held by the team (subject `team:{slug}`) and flows to every
/// member when their effective grants are resolved. Same fail-closed authority
/// rule as [`create_grant`]: the grantor must be an admin in the team's
/// namespace.
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `grantor` lacks authority, or
/// [`GateError::GrantStore`] if the write fails.
pub async fn create_team_grant(
    db: &Surreal<Db>,
    grantor: &Principal,
    slug: &str,
    namespace: &str,
    capability: Capability,
) -> Result<Grant> {
    write_grant(db, grantor, Grant::for_team(slug, namespace, capability)).await
}

/// Authority-check `grant` against `grantor` and upsert it (fail closed).
///
/// Shared by the principal and team grant paths: the only difference between
/// them is the grant's subject, so both run the same `may_administer` check
/// (which keys off the grant's namespace, not its subject) and the same write.
pub(super) async fn write_grant(
    db: &Surreal<Db>,
    grantor: &Principal,
    grant: Grant,
) -> Result<Grant> {
    if !may_administer(grantor, &grant) {
        return Err(GateError::GrantDenied(format!(
            "{} may not grant {} to {} in namespace {}",
            grantor.subject,
            grant.capability.as_str(),
            grant.subject,
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
