//! Read, delete, and role-change verbs over the gate-owned `principal` table.
//!
//! `provision_principal` (the seed/library path) and `authenticate` are the only
//! principal verbs that exist today; the `principal` table is otherwise opaque to
//! callers (`PrincipalRow` is `pub(crate)`). The HTTP admin surface
//! (`rubix/docs/design/ADMIN-API.md`, Surface 1) needs to list, fetch, delete,
//! and re-band identities, so this module adds those verbs **without** opening the
//! row type — they return the public `rubix_core::Principal` and never the secret.
//!
//! Every mutation here (`create`/`delete`/`set_role`) appends an immutable audit
//! row stamped with a fresh correlation id, the same accountability the record
//! command path takes (`rubix/docs/design/ADMIN-API.md`, "the gate boundary").
//! The `principal` table is not the generic `record` table, so these do not flow
//! through [`apply`](crate::apply); they write the table directly and audit by the
//! same internal [`append_audit`] the command pipeline uses.
//!
//! Subjects here are **full** subjects (the prefixed `{namespace}_{local}` key the
//! seed already uses); the API-local/full mapping is the transport's concern.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};

use crate::audit::{AuditRecord, append_audit};
use crate::command::CapturedChange;
use crate::error::{GateError, Result};

use super::provision::provision_principal;
use super::row::{PRINCIPAL_TABLE, PrincipalRow};

/// The audit target prefix for a principal mutation (distinct from `record:`).
fn audit_target(subject: &str) -> Id {
    Id::from_raw(format!("{PRINCIPAL_TABLE}:{subject}"))
}

/// Provision `principal` with `secret` and append a `create` audit row.
///
/// The audited counterpart of [`provision_principal`](super::provision_principal):
/// the seed path stays unaudited (it provisions before any audit schema exists),
/// while the HTTP admin path routes through here so every identity creation is
/// accountable (`rubix/docs/design/ADMIN-API.md`, Surface 1). Provision is
/// non-idempotent — re-creating an existing subject surfaces the store's
/// duplicate-key error, which the transport maps to `409`.
///
/// # Errors
/// Returns [`GateError::IssueSession`] if the identity write fails (including a
/// duplicate subject), or [`GateError::AuditWrite`] if the audit append fails.
pub async fn create_principal(
    db: &Surreal<Db>,
    actor: &Principal,
    principal: &Principal,
    secret: impl Into<String>,
) -> Result<()> {
    provision_principal(db, principal, secret).await?;
    let captured = CapturedChange {
        before: None,
        after: Some(principal_summary(principal)),
    };
    audit(db, actor, "create", principal.subject.as_str(), &captured).await
}

/// List every principal in `namespace`, secrets stripped.
///
/// Runs on the root handle: the `principal` table has no scoped read permission,
/// so listing is an owner-only verb the admin transport guards with its
/// admin-in-namespace rule before calling.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the query fails.
pub async fn list_principals(db: &Surreal<Db>, namespace: &str) -> Result<Vec<Principal>> {
    let query = format!("SELECT * FROM {PRINCIPAL_TABLE} WHERE namespace = $namespace");
    let mut response = db
        .query(query)
        .bind(("namespace", namespace.to_owned()))
        .await
        .map_err(GateError::Lookup)?;
    let rows: Vec<PrincipalRow> = response.take(0).map_err(GateError::Lookup)?;
    Ok(rows.into_iter().filter_map(PrincipalRow::into_principal).collect())
}

/// Fetch one principal by its full `subject`, scoped to `namespace`, or `None`.
///
/// Filters by namespace so a subject from another tenant is invisible even though
/// the `principal` table is keyed globally. The secret is dropped.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the read fails.
pub async fn get_principal(
    db: &Surreal<Db>,
    namespace: &str,
    subject: &str,
) -> Result<Option<Principal>> {
    let row: Option<PrincipalRow> = db
        .select((PRINCIPAL_TABLE, subject))
        .await
        .map_err(GateError::Lookup)?;
    Ok(row
        .filter(|row| row.namespace == namespace)
        .and_then(PrincipalRow::into_principal))
}

/// Delete the principal `subject` in `namespace`, appending an audit row.
///
/// `actor` is the principal performing the deletion (the audit subject). The
/// before-image is the deleted row's identity (secret excluded); the after-image
/// is `None`. Deleting an absent subject is a no-op that still audits the attempt
/// against an empty before-image — callers that need a `404` check existence with
/// [`get_principal`] first (the transport does, for the last-admin guard).
///
/// # Errors
/// Returns [`GateError::Lookup`] if the delete fails or
/// [`GateError::AuditWrite`] if the audit append fails.
pub async fn delete_principal(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
    subject: &str,
) -> Result<()> {
    let before = get_principal(db, namespace, subject).await?;
    let _: Option<PrincipalRow> = db
        .delete((PRINCIPAL_TABLE, subject))
        .await
        .map_err(GateError::Lookup)?;
    let captured = CapturedChange {
        before: before.as_ref().map(principal_summary),
        after: None,
    };
    audit(db, actor, "delete", subject, &captured).await
}

/// Set the principal `subject`'s role in `namespace`, returning the updated
/// principal and appending an audit row.
///
/// Re-bands an existing identity in place (the access secret is untouched).
/// Returns [`GateError::Authenticate`] if the subject does not exist in the
/// namespace — the transport maps that to `404`.
///
/// # Errors
/// Returns [`GateError::Lookup`] on a store failure, [`GateError::Authenticate`]
/// for an unknown subject, or [`GateError::AuditWrite`] if the audit append fails.
pub async fn set_principal_role(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
    subject: &str,
    role: rubix_core::Role,
) -> Result<Principal> {
    let before = get_principal(db, namespace, subject)
        .await?
        .ok_or_else(|| GateError::Authenticate("unknown principal".to_owned()))?;
    let role_str = role_str(role);
    let _: Option<PrincipalRow> = db
        .query(format!(
            "UPDATE type::record('{PRINCIPAL_TABLE}', $subject) SET role = $role"
        ))
        .bind(("subject", subject.to_owned()))
        .bind(("role", role_str.to_owned()))
        .await
        .map_err(GateError::Lookup)?
        .take(0)
        .map_err(GateError::Lookup)?;
    let after = Principal::new(before.subject.clone(), namespace.to_owned(), before.kind, role);
    let captured = CapturedChange {
        before: Some(principal_summary(&before)),
        after: Some(principal_summary(&after)),
    };
    audit(db, actor, "update", subject, &captured).await?;
    Ok(after)
}

/// Append a principal-mutation audit row stamped with a fresh correlation id.
async fn audit(
    db: &Surreal<Db>,
    actor: &Principal,
    action: &str,
    subject: &str,
    captured: &CapturedChange,
) -> Result<()> {
    let correlation_id = CorrelationId::mint();
    let record = AuditRecord::project(actor, action, &audit_target(subject), captured, &correlation_id);
    append_audit(db, &record).await
}

/// The audit before/after summary of a principal — identity only, never a secret.
fn principal_summary(principal: &Principal) -> serde_json::Value {
    serde_json::json!({
        "subject": principal.subject.to_string(),
        "namespace": principal.namespace,
        "kind": kind_str(principal.kind),
        "role": role_str(principal.role),
    })
}

fn kind_str(kind: rubix_core::PrincipalKind) -> &'static str {
    match kind {
        rubix_core::PrincipalKind::User => "user",
        rubix_core::PrincipalKind::Extension => "extension",
    }
}

fn role_str(role: rubix_core::Role) -> &'static str {
    match role {
        rubix_core::Role::Viewer => "viewer",
        rubix_core::Role::Operator => "operator",
        rubix_core::Role::Admin => "admin",
    }
}
