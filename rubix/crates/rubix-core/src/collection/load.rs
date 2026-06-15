//! Load collection definitions and the namespace strict-mode switch from the store.
//!
//! Collections and namespace settings are ordinary records
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "a record, not a table"), so these
//! loaders are plain `SELECT`s over the `record` table on the gate's root handle,
//! filtered by namespace. They are read by the gate's validate step to resolve a
//! write's `kind` to its contract and to decide whether an unknown `kind` is
//! admitted (fail-open) or rejected (strict mode) — the decision that makes
//! validation real rather than cosmetic (open question 1).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::def::{COLLECTION_KIND, CollectionDef, CollectionParseError};

/// The `kind` of the per-namespace settings record carrying the strict-mode flag.
///
/// Underscore-prefixed so it never collides with a tenant collection name; one
/// such record per namespace holds platform switches that are not tenant data.
pub const NAMESPACE_SETTINGS_KIND: &str = "_namespace_settings";

/// Find the collection in `namespace` whose `name` matches `kind`.
///
/// Returns `Ok(None)` when no collection record names `kind` — the write is then
/// unconstrained (fail-open) unless the namespace is in strict mode. A collection
/// record that exists but cannot be parsed is surfaced as an error, not silently
/// skipped: a malformed contract must not degrade to "no contract".
///
/// # Errors
/// Returns [`Error::Store`] if the query fails, or [`Error::Store`] wrapping a
/// [`CollectionParseError`] if the matched record is not a valid definition.
pub async fn find_collection(
    db: &Surreal<Db>,
    namespace: &str,
    kind: &str,
) -> Result<Option<CollectionDef>> {
    let content: Option<serde_json::Value> = db
        .query(
            "SELECT VALUE content FROM record \
             WHERE namespace = $namespace \
               AND content.kind = $collection_kind \
               AND content.name = $name \
             LIMIT 1",
        )
        .bind(("namespace", namespace.to_owned()))
        .bind(("collection_kind", COLLECTION_KIND))
        .bind(("name", kind.to_owned()))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;

    match content {
        None => Ok(None),
        Some(value) => CollectionDef::parse(&value)
            .map(Some)
            .map_err(|e: CollectionParseError| {
                Error::Store(format!("collection `{kind}` is malformed: {e}"))
            }),
    }
}

/// Whether `namespace` is in strict mode (unknown `kind`s rejected).
///
/// Reads the namespace settings record's `strict` flag, defaulting to `false`
/// (fail-open) when no settings record exists — a fresh namespace keeps today's
/// behaviour so existing unkinded records still write. Flipping to strict is a
/// deliberate per-namespace act (writing the settings record), the intended end
/// state once a tenant's collections exist.
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn namespace_strict(db: &Surreal<Db>, namespace: &str) -> Result<bool> {
    let strict: Option<bool> = db
        .query(
            "SELECT VALUE content.strict FROM record \
             WHERE namespace = $namespace \
               AND content.kind = $settings_kind \
             LIMIT 1",
        )
        .bind(("namespace", namespace.to_owned()))
        .bind(("settings_kind", NAMESPACE_SETTINGS_KIND))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(strict.unwrap_or(false))
}
