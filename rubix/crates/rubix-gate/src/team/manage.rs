//! Gate verbs over the `team` and `membership` tables — authority-checked, audited.
//!
//! Teams are a gate-owned identity primitive (`rubix/docs/SCOPE.md`, principle
//! 5), administered the same way grants are: every mutation here requires the
//! actor to be an [`Admin`](rubix_core::Role::Admin) in the team's namespace
//! (the same `may_administer` rule the grant layer uses, fail-closed and no
//! cross-tenant write), and every mutation appends an immutable audit row stamped
//! with a fresh correlation id. The `team`/`membership` tables are not the generic
//! `record` table, so these write the tables directly and audit by the same
//! internal [`append_audit`] the command pipeline uses.
//!
//! Subjects here are **full** subjects (the prefixed `{namespace}_{local}` key),
//! matching the grant and principal layers; the API-local mapping is the
//! transport's concern. `teams_of` is the key resolution the authz layers build
//! on: it returns the slugs a principal belongs to, so a grant made to a team can
//! flow to its members.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal, Role};

use crate::audit::{AuditRecord, append_audit};
use crate::command::CapturedChange;
use crate::error::{GateError, Result};

use super::model::{Membership, Team};
use super::row::{MEMBERSHIP_TABLE, MembershipRow, TEAM_TABLE, TeamRow, membership_key, team_key};

/// Whether `actor` may administer teams/memberships in `namespace`.
///
/// The same rule the grant layer enforces: an admin operating in its own
/// namespace, fail-closed. A non-admin, or an admin reaching across a tenant
/// boundary, is denied.
fn may_administer(actor: &Principal, namespace: &str) -> bool {
    actor.role == Role::Admin && actor.namespace == namespace
}

/// Refuse the operation unless `actor` may administer teams in `namespace`.
fn require_authority(actor: &Principal, namespace: &str, action: &str) -> Result<()> {
    if may_administer(actor, namespace) {
        Ok(())
    } else {
        Err(GateError::GrantDenied(format!(
            "{} may not {action} a team in namespace {namespace}",
            actor.subject
        )))
    }
}

/// Create `team`, authorized by `actor`, and append a `create` audit row.
///
/// Idempotent on the deterministic `{namespace}:{slug}` key — re-creating a team
/// upserts its display name rather than erroring, mirroring the grant layer.
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `actor` lacks authority,
/// [`GateError::GrantStore`] if the write fails, or [`GateError::AuditWrite`] if
/// the audit append fails.
pub async fn create_team(db: &Surreal<Db>, actor: &Principal, team: &Team) -> Result<Team> {
    require_authority(actor, &team.namespace, "create")?;
    let row = TeamRow::from_team(team);
    let _: Option<TeamRow> = db
        .upsert((TEAM_TABLE, team_key(&team.namespace, &team.slug)))
        .content(row)
        .await
        .map_err(GateError::GrantStore)?;
    let captured = CapturedChange {
        before: None,
        after: Some(team_summary(team)),
    };
    audit(
        db,
        actor,
        "create",
        &team_target(&team.namespace, &team.slug),
        &captured,
    )
    .await?;
    Ok(team.clone())
}

/// List every team in `namespace`.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the query fails.
pub async fn list_teams(db: &Surreal<Db>, namespace: &str) -> Result<Vec<Team>> {
    let query = format!("SELECT * FROM {TEAM_TABLE} WHERE namespace = $namespace");
    let mut response = db
        .query(query)
        .bind(("namespace", namespace.to_owned()))
        .await
        .map_err(GateError::Lookup)?;
    let rows: Vec<TeamRow> = response.take(0).map_err(GateError::Lookup)?;
    Ok(rows.into_iter().map(TeamRow::into_team).collect())
}

/// Fetch one team by `slug`, scoped to `namespace`, or `None`.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the read fails.
pub async fn get_team(db: &Surreal<Db>, namespace: &str, slug: &str) -> Result<Option<Team>> {
    let row: Option<TeamRow> = db
        .select((TEAM_TABLE, team_key(namespace, slug)))
        .await
        .map_err(GateError::Lookup)?;
    Ok(row
        .filter(|r| r.namespace == namespace)
        .map(TeamRow::into_team))
}

/// Delete the team `slug` in `namespace` and all its memberships, audited.
///
/// Removing a team also removes every membership in it, so no membership is left
/// pointing at a team that no longer exists. Deleting an absent team is a no-op
/// that still audits the attempt; callers that need a `404` check existence with
/// [`get_team`] first.
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `actor` lacks authority,
/// [`GateError::GrantStore`]/[`GateError::Lookup`] if a store op fails, or
/// [`GateError::AuditWrite`] if the audit append fails.
pub async fn delete_team(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
    slug: &str,
) -> Result<()> {
    require_authority(actor, namespace, "delete")?;
    let before = get_team(db, namespace, slug).await?;
    let _: Option<TeamRow> = db
        .delete((TEAM_TABLE, team_key(namespace, slug)))
        .await
        .map_err(GateError::GrantStore)?;
    // Drop every membership of the removed team so none dangles.
    db.query(format!(
        "DELETE {MEMBERSHIP_TABLE} WHERE namespace = $namespace AND team_slug = $slug"
    ))
    .bind(("namespace", namespace.to_owned()))
    .bind(("slug", slug.to_owned()))
    .await
    .map_err(GateError::GrantStore)?;
    let captured = CapturedChange {
        before: before.as_ref().map(team_summary),
        after: None,
    };
    audit(
        db,
        actor,
        "delete",
        &team_target(namespace, slug),
        &captured,
    )
    .await
}

/// Add `subject` to the team `slug` in `namespace`, audited (idempotent).
///
/// `subject` is a full storage subject. The membership key is deterministic, so
/// adding the same member twice upserts the one row. The caller is responsible
/// for confirming the team and the principal exist (the transport does), since
/// this verb writes the link unconditionally.
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `actor` lacks authority,
/// [`GateError::GrantStore`] if the write fails, or [`GateError::AuditWrite`] if
/// the audit append fails.
pub async fn add_member(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
    slug: &str,
    subject: &str,
) -> Result<Membership> {
    require_authority(actor, namespace, "add a member to")?;
    let membership = Membership::new(namespace, slug, subject);
    let row = MembershipRow::from_membership(&membership);
    let _: Option<MembershipRow> = db
        .upsert((MEMBERSHIP_TABLE, membership_key(namespace, slug, subject)))
        .content(row)
        .await
        .map_err(GateError::GrantStore)?;
    let captured = CapturedChange {
        before: None,
        after: Some(membership_summary(&membership)),
    };
    audit(
        db,
        actor,
        "create",
        &membership_target(&membership),
        &captured,
    )
    .await?;
    Ok(membership)
}

/// Remove `subject` from the team `slug` in `namespace`, audited (idempotent).
///
/// # Errors
/// Returns [`GateError::GrantDenied`] if `actor` lacks authority,
/// [`GateError::GrantStore`] if the delete fails, or [`GateError::AuditWrite`] if
/// the audit append fails.
pub async fn remove_member(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
    slug: &str,
    subject: &str,
) -> Result<()> {
    require_authority(actor, namespace, "remove a member from")?;
    let _: Option<MembershipRow> = db
        .delete((MEMBERSHIP_TABLE, membership_key(namespace, slug, subject)))
        .await
        .map_err(GateError::GrantStore)?;
    let membership = Membership::new(namespace, slug, subject);
    let captured = CapturedChange {
        before: Some(membership_summary(&membership)),
        after: None,
    };
    audit(
        db,
        actor,
        "delete",
        &membership_target(&membership),
        &captured,
    )
    .await
}

/// List the full subjects of every member of the team `slug` in `namespace`.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the query fails.
pub async fn list_members(db: &Surreal<Db>, namespace: &str, slug: &str) -> Result<Vec<String>> {
    let query = format!(
        "SELECT VALUE subject FROM {MEMBERSHIP_TABLE} \
         WHERE namespace = $namespace AND team_slug = $slug"
    );
    let mut response = db
        .query(query)
        .bind(("namespace", namespace.to_owned()))
        .bind(("slug", slug.to_owned()))
        .await
        .map_err(GateError::Lookup)?;
    let subjects: Vec<String> = response.take(0).map_err(GateError::Lookup)?;
    Ok(subjects)
}

/// Return the slugs of every team `principal` belongs to in its namespace.
///
/// The resolution the authz layers build on: a grant made to a team flows to its
/// members because [`list_grants`](crate::list_grants) and
/// [`check_capability`](crate::check_capability) union a principal's own grants
/// with the grants of the teams this returns. Scoped to the principal's namespace
/// so membership never crosses a tenant boundary.
///
/// # Errors
/// Returns [`GateError::Lookup`] if the query fails.
pub async fn teams_of(db: &Surreal<Db>, principal: &Principal) -> Result<Vec<String>> {
    let query = format!(
        "SELECT VALUE team_slug FROM {MEMBERSHIP_TABLE} \
         WHERE namespace = $namespace AND subject = $subject"
    );
    let mut response = db
        .query(query)
        .bind(("namespace", principal.namespace.clone()))
        .bind(("subject", principal.subject.to_string()))
        .await
        .map_err(GateError::Lookup)?;
    let slugs: Vec<String> = response.take(0).map_err(GateError::Lookup)?;
    Ok(slugs)
}

/// Append a team/membership-mutation audit row stamped with a fresh correlation id.
async fn audit(
    db: &Surreal<Db>,
    actor: &Principal,
    action: &str,
    target: &Id,
    captured: &CapturedChange,
) -> Result<()> {
    let correlation_id = CorrelationId::mint();
    let record = AuditRecord::project(actor, action, target, captured, &correlation_id);
    append_audit(db, &record).await
}

/// The audit target id for a team mutation (distinct from `record:`/`grant:`).
fn team_target(namespace: &str, slug: &str) -> Id {
    Id::from_raw(format!("{TEAM_TABLE}:{}", team_key(namespace, slug)))
}

/// The audit target id for a membership mutation.
fn membership_target(membership: &Membership) -> Id {
    Id::from_raw(format!(
        "{MEMBERSHIP_TABLE}:{}",
        membership_key(
            &membership.namespace,
            &membership.team_slug,
            &membership.subject
        )
    ))
}

/// The audit before/after summary of a team.
fn team_summary(team: &Team) -> serde_json::Value {
    serde_json::json!({
        "slug": team.slug,
        "namespace": team.namespace,
        "display_name": team.display_name,
    })
}

/// The audit before/after summary of a membership.
fn membership_summary(membership: &Membership) -> serde_json::Value {
    serde_json::json!({
        "namespace": membership.namespace,
        "team_slug": membership.team_slug,
        "subject": membership.subject,
    })
}
