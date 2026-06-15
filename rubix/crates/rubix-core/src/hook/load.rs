//! Load hook bindings for a namespace from the record store.
//!
//! Hooks are ordinary `kind: "hook"` records, so this is a plain `SELECT` over
//! the `record` table on the gate's root handle, filtered by namespace. The
//! write-trigger dispatcher reads them to decide which rules a write fires
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Server-side hooks").

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::def::{HOOK_KIND, Hook};

/// Load every well-formed hook binding defined in `namespace`.
///
/// Malformed hook records are skipped rather than failing the whole load — one
/// bad binding must not silence every other hook. (A stricter policy belongs to
/// the validate path, where a `hook` collection could reject malformed bindings
/// at write time.)
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn find_hooks(db: &Surreal<Db>, namespace: &str) -> Result<Vec<Hook>> {
    let contents: Vec<serde_json::Value> = db
        .query(
            "SELECT VALUE content FROM record \
             WHERE namespace = $namespace AND content.kind = $hook_kind",
        )
        .bind(("namespace", namespace.to_owned()))
        .bind(("hook_kind", HOOK_KIND))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;

    Ok(contents
        .iter()
        .filter_map(|content| Hook::parse(content).ok())
        .collect())
}
