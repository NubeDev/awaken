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

use super::model::Grant;
use super::row::{GRANT_TABLE, GrantRow};

/// Return every grant held by `principal`, scoped to its namespace.
///
/// # Errors
/// Returns [`GateError::GrantStore`] if the query fails.
pub async fn list_grants(db: &Surreal<Db>, principal: &Principal) -> Result<Vec<Grant>> {
    let query = format!(
        "SELECT * FROM {GRANT_TABLE} WHERE subject = $subject AND namespace = $namespace"
    );
    let mut response = db
        .query(query)
        .bind(("subject", principal.subject.to_string()))
        .bind(("namespace", principal.namespace.clone()))
        .await
        .map_err(GateError::GrantStore)?;
    let rows: Vec<GrantRow> = response.take(0).map_err(GateError::GrantStore)?;
    Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
}
