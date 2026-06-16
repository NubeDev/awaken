//! The contract-validation step: enforce a collection's shape on a write.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`) puts every mutation through the gate, so
//! the per-kind validation the collection layer needs goes here — *after*
//! [`authorize`](super::authorize), *before* [`capture`](super::capture) — and
//! nowhere else (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Per-kind validation
//! in the gate write path"). Putting it in the HTTP handler would let the agent
//! runtime, rules, sync, and ingest write unvalidated content; one enforcement
//! point keeps the contract real.
//!
//! Resolution is by the write's `content.kind`: load the collection of that name,
//! validate the content against it. No matching collection is **fail-open**
//! (admitted as raw JSON, today's behaviour) unless the namespace is in **strict
//! mode**, where an unknown/typo'd kind is rejected — the switch that decides
//! whether validation is real or cosmetic (open question 1). A delete carries no
//! content, so it is never validated here.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{find_collection, namespace_strict, read_record};
use serde_json::Value;

use crate::error::{GateError, Result};

use super::action::Change;
use super::define::Command;

/// Validate `command`'s content against its collection, if any.
///
/// For a create the new content is checked directly; for an update the content
/// is the existing record merged with the update (top-level keys), so a partial
/// update is validated against the record it produces, not the patch alone. A
/// delete is a no-op. The merge mirrors SurrealDB `MERGE` at the field level,
/// which is where collection fields live.
///
/// # Errors
/// Returns [`GateError::Validation`] if the content fails its collection's
/// schema, or if the namespace is strict and the `kind` matches no collection.
/// Returns [`GateError::Read`] if loading the collection or the existing record
/// for a merge fails.
pub(crate) async fn validate(db: &Surreal<Db>, command: &Command) -> Result<()> {
    let content = match &command.change {
        Change::Create(content) => content.clone(),
        Change::Update(content) => merged_for_update(db, command, content).await?,
        Change::Delete => return Ok(()),
    };

    let namespace = command.namespace();
    let kind = content.get("kind").and_then(Value::as_str);

    let collection = match kind {
        Some(kind) => find_collection(db, namespace, kind)
            .await
            .map_err(GateError::Read)?,
        None => None,
    };

    match collection {
        Some(def) => def
            .validate(&content)
            .map_err(|e| GateError::Validation(e.to_string())),
        None => {
            if namespace_strict(db, namespace)
                .await
                .map_err(GateError::Read)?
            {
                Err(GateError::Validation(format!(
                    "namespace `{namespace}` is in strict mode and `{}` matches no collection",
                    kind.unwrap_or("<no kind>")
                )))
            } else {
                Ok(())
            }
        }
    }
}

/// Produce the content an update yields: the existing record merged with the patch.
///
/// Reads the target on the root handle and overlays the patch's top-level keys.
/// When the target does not yet exist the patch stands alone — an update to a
/// missing record validates the patch as written (SurrealDB applies no merge in
/// that case either).
async fn merged_for_update(db: &Surreal<Db>, command: &Command, patch: &Value) -> Result<Value> {
    let existing = read_record(db, &command.target)
        .await
        .map_err(GateError::Read)?;

    match existing.map(|record| record.content) {
        Some(Value::Object(mut base)) => {
            if let Some(patch_obj) = patch.as_object() {
                for (key, value) in patch_obj {
                    base.insert(key.clone(), value.clone());
                }
            }
            Ok(Value::Object(base))
        }
        // No existing object to merge onto — validate the patch as written.
        _ => Ok(patch.clone()),
    }
}
