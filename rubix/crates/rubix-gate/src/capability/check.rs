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
use super::kind::Capability;
use super::register::is_registered;

/// Whether `principal` may exercise `capability`.
///
/// Denies (returns `false`) when the capability is not registered or when the
/// principal holds no matching grant in its namespace. Allows only when a grant
/// for that exact (subject, namespace, capability) triple exists.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if the grant lookup itself fails. A query
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
    let query = format!(
        "SELECT VALUE count() FROM {GRANT_TABLE} \
         WHERE subject = $subject AND namespace = $namespace AND capability = $capability \
         GROUP ALL"
    );
    let mut response = db
        .query(query)
        .bind(("subject", principal.subject.to_string()))
        .bind(("namespace", principal.namespace.clone()))
        .bind(("capability", capability.as_str().to_owned()))
        .await
        .map_err(GateError::GrantStore)?;
    let matches: Option<i64> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(matches.unwrap_or(0) > 0)
}
