//! List the capability grants held by a principal.
//!
//! Grants are app-enforced, so the gate reads them on the store handle and
//! filters by the principal's subject *and* namespace — a grant only counts for
//! the identity in its own tenant (`rubix/docs/SCOPE.md`, "Two authz layers").
//! Rows carrying an unknown capability are dropped, never guessed, so a stale
//! row can never surface as a usable grant.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::error::{GateError, Result};
use crate::team::teams_of;

use super::model::{Grant, team_subject};
use super::row::{GRANT_TABLE, GrantRow};

/// Return every grant held **directly** by `principal`, scoped to its namespace.
///
/// This is the principal's *own* grants only — it does not fold in grants the
/// principal inherits through team membership. The admin surface uses it to show
/// what was granted to one identity; for the inherited set use
/// [`effective_grants`], and note the enforcement point
/// ([`check_capability`](crate::check_capability)) already unions teams.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if the query fails.
pub async fn list_grants(db: &Surreal<Db>, principal: &Principal) -> Result<Vec<Grant>> {
    let query =
        format!("SELECT * FROM {GRANT_TABLE} WHERE subject = $subject AND namespace = $namespace");
    let mut response = db
        .query(query)
        .bind(("subject", principal.subject.to_string()))
        .bind(("namespace", principal.namespace.clone()))
        .await
        .map_err(GateError::GrantStore)?;
    let rows: Vec<GrantRow> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
}

/// Return every grant the team `slug` holds in `namespace`.
///
/// The grants attached to the team subject (`team:{slug}`) — the set every
/// member inherits.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if the query fails.
pub async fn list_team_grants(db: &Surreal<Db>, namespace: &str, slug: &str) -> Result<Vec<Grant>> {
    let query =
        format!("SELECT * FROM {GRANT_TABLE} WHERE subject = $subject AND namespace = $namespace");
    let mut response = db
        .query(query)
        .bind(("subject", team_subject(slug)))
        .bind(("namespace", namespace.to_owned()))
        .await
        .map_err(GateError::GrantStore)?;
    let rows: Vec<GrantRow> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
}

/// Return `principal`'s **effective** grants: its own grants unioned with the
/// grants of every team it belongs to.
///
/// This is the set [`check_capability`](crate::check_capability) enforces against
/// — the inherited view. Duplicates (the same capability held directly and via a
/// team) are not de-duplicated by capability here; each row is returned as
/// stored, so a caller that needs a set should fold by capability.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if a grant query fails, or
/// [`GateError::Lookup`] if resolving the principal's teams fails.
pub async fn effective_grants(db: &Surreal<Db>, principal: &Principal) -> Result<Vec<Grant>> {
    let mut subjects = vec![principal.subject.to_string()];
    subjects.extend(
        teams_of(db, principal)
            .await?
            .iter()
            .map(|s| team_subject(s)),
    );

    let query = format!(
        "SELECT * FROM {GRANT_TABLE} WHERE subject IN $subjects AND namespace = $namespace"
    );
    let mut response = db
        .query(query)
        .bind(("subjects", subjects))
        .bind(("namespace", principal.namespace.clone()))
        .await
        .map_err(GateError::GrantStore)?;
    let rows: Vec<GrantRow> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
}
