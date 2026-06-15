//! Audited grant administration for the HTTP admin surface.
//!
//! [`create_grant`](super::create_grant) and [`revoke_grant`](super::revoke_grant)
//! are the seed/library path: authority-checked, fail-closed, but unaudited (the
//! seed grants before any audit schema exists). The HTTP admin surface
//! (`rubix/docs/design/ADMIN-API.md`, Surface 2) routes through these wrappers so
//! every grant change is accountable — same authority check, plus an immutable
//! audit row stamped with a fresh correlation id, the same boundary the record
//! command path takes.
//!
//! The grant table is not the generic `record` table, so these do not flow
//! through [`apply`](crate::apply); they reuse the existing grant verbs (which own
//! the `may_administer` check) and append audit by the same internal
//! [`append_audit`] the command pipeline uses.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};

use crate::audit::{AuditRecord, append_audit};
use crate::capability::kind::Capability;
use crate::command::CapturedChange;
use crate::error::Result;

use super::create::create_grant;
use super::model::Grant;
use super::revoke::revoke_grant;
use super::row::GRANT_TABLE;

/// Grant `capability` to `grantee`, authorized by `grantor`, and audit it.
///
/// The audited counterpart of [`create_grant`](super::create_grant): the same
/// fail-closed `may_administer` check runs first (the write never happens for an
/// unauthorized grantor, so no audit row is produced for a denial), then a
/// `create` audit row is appended.
///
/// # Errors
/// Returns [`GateError::GrantDenied`](crate::GateError::GrantDenied) if `grantor`
/// lacks authority, [`GateError::GrantStore`](crate::GateError::GrantStore) if the
/// write fails, or [`GateError::AuditWrite`](crate::GateError::AuditWrite) if the
/// audit append fails.
pub async fn create_grant_audited(
    db: &Surreal<Db>,
    grantor: &Principal,
    grantee: &Principal,
    capability: Capability,
) -> Result<Grant> {
    let grant = create_grant(db, grantor, grantee, capability).await?;
    let captured = CapturedChange {
        before: None,
        after: Some(grant_summary(&grant)),
    };
    audit(db, grantor, "create", &grant, &captured).await?;
    Ok(grant)
}

/// Revoke `capability` from `grantee`, authorized by `grantor`, and audit it.
///
/// The audited counterpart of [`revoke_grant`](super::revoke_grant). Revoke is
/// idempotent, so the audit records the intent even when the grant was already
/// absent (before-image is the grant identity, after-image is `None`).
///
/// # Errors
/// Returns [`GateError::GrantDenied`](crate::GateError::GrantDenied) if `grantor`
/// lacks authority, [`GateError::GrantStore`](crate::GateError::GrantStore) if the
/// delete fails, or [`GateError::AuditWrite`](crate::GateError::AuditWrite) if the
/// audit append fails.
pub async fn revoke_grant_audited(
    db: &Surreal<Db>,
    grantor: &Principal,
    grantee: &Principal,
    capability: Capability,
) -> Result<()> {
    revoke_grant(db, grantor, grantee, capability).await?;
    let grant = Grant::new(grantee, capability);
    let captured = CapturedChange {
        before: Some(grant_summary(&grant)),
        after: None,
    };
    audit(db, grantor, "delete", &grant, &captured).await
}

/// Append a grant-mutation audit row stamped with a fresh correlation id.
async fn audit(
    db: &Surreal<Db>,
    actor: &Principal,
    action: &str,
    grant: &Grant,
    captured: &CapturedChange,
) -> Result<()> {
    let correlation_id = CorrelationId::mint();
    let target = Id::from_raw(format!(
        "{GRANT_TABLE}:{}:{}:{}",
        grant.namespace,
        grant.subject,
        grant.capability.as_str()
    ));
    let record = AuditRecord::project(actor, action, &target, captured, &correlation_id);
    append_audit(db, &record).await
}

/// The audit before/after summary of a grant.
fn grant_summary(grant: &Grant) -> serde_json::Value {
    serde_json::json!({
        "subject": grant.subject,
        "namespace": grant.namespace,
        "capability": grant.capability.as_str(),
    })
}
