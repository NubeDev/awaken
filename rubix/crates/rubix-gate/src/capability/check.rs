//! `check_capability` — the app-enforced allow/deny decision, fail closed.
//!
//! This is the enforcement point for the second authz layer
//! (`rubix/docs/SCOPE.md`, "Two authz layers"): given a principal and a
//! capability, decide whether the principal may perform the action. The decision
//! is app-enforced (it governs cross-plane actions SurrealDB does not see), and
//! it **fails closed** — a capability the registry does not know and a principal
//! without a matching grant both deny. A grant only counts within the
//! principal's own namespace, mirroring the row-level read scope of WS-03.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::error::{GateError, Result};

use super::grant::row::GRANT_TABLE;
use super::grant::team_subject;
use super::kind::Capability;
use super::register::is_registered;
use crate::team::teams_of;

/// Whether `principal` may exercise `capability`.
///
/// Denies (returns `false`) when the capability is not registered or when
/// neither the principal nor any team it belongs to holds a matching grant in
/// its namespace. A principal's **effective** grants are its own grants unioned
/// with the grants of every team it is a member of (`rubix/docs/SCOPE.md`,
/// "Capabilities are grants"; team inheritance in [`team`](crate::team)), so a
/// capability granted to a team is exercisable by each member.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if the grant lookup itself fails, or
/// [`GateError::Lookup`] if resolving the principal's teams fails. A query
/// failure is surfaced, never silently treated as allow.
pub async fn check_capability(
    db: &Surreal<Db>,
    principal: &Principal,
    capability: Capability,
) -> Result<bool> {
    // Fail closed on an unknown capability before consulting any grant.
    if !is_registered(capability.as_str()) {
        return Ok(false);
    }
    // The principal's own subject plus a `team:{slug}` subject for each team it
    // belongs to — the set of subjects whose grants apply to this principal.
    let mut subjects = vec![principal.subject.to_string()];
    subjects.extend(
        teams_of(db, principal)
            .await?
            .iter()
            .map(|s| team_subject(s)),
    );

    let query = format!(
        "SELECT VALUE count() FROM {GRANT_TABLE} \
         WHERE subject IN $subjects AND namespace = $namespace AND capability = $capability \
         GROUP ALL"
    );
    let mut response = db
        .query(query)
        .bind(("subjects", subjects))
        .bind(("namespace", principal.namespace.clone()))
        .bind(("capability", capability.as_str().to_owned()))
        .await
        .map_err(GateError::GrantStore)?;
    let matches: Option<i64> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(matches.unwrap_or(0) > 0)
}
