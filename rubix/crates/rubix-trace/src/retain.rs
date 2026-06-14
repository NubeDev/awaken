//! Enforce the rolling retention bound on the `trace` table.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Tracing"):
//! traces are bounded — high volume, not kept forever. This verb caps a
//! namespace's stored spans at `max_spans`, evicting the oldest by store append
//! time once the cap is exceeded. Eviction runs on the root/owner handle (the
//! only session past the `trace` table's `FOR delete NONE` permission); the spans
//! themselves are never mutated, so append-only-ness holds for callers while the
//! system rolls the bound.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Result, TraceError};
use crate::row::TRACE_TABLE;

/// Evict the oldest spans in `namespace` beyond the `max_spans` bound.
///
/// Counts the namespace's spans; if the count is at or under `max_spans`, this is
/// a no-op. Otherwise the `excess` oldest spans (by `appended`, ties broken by
/// id) are deleted, leaving exactly `max_spans` most-recent spans. Returns the
/// number of spans evicted.
///
/// A `max_spans` of `0` evicts every span in the namespace — a valid "retain
/// nothing" bound.
///
/// # Errors
/// Returns [`TraceError::Retain`] if counting or deleting fails.
pub async fn enforce_retention(
    db: &Surreal<Db>,
    namespace: &str,
    max_spans: usize,
) -> Result<usize> {
    let count = count_spans(db, namespace).await?;
    if count <= max_spans {
        return Ok(0);
    }
    let excess = count - max_spans;
    delete_oldest(db, namespace, excess).await?;
    Ok(excess)
}

/// Count the spans stored for `namespace`.
async fn count_spans(db: &Surreal<Db>, namespace: &str) -> Result<usize> {
    let counts: Vec<i64> = db
        .query(format!(
            "SELECT VALUE count() FROM {TRACE_TABLE} WHERE namespace = $ns GROUP ALL"
        ))
        .bind(("ns", namespace.to_owned()))
        .await
        .map_err(TraceError::Retain)?
        .take(0)
        .map_err(TraceError::Retain)?;
    #[allow(clippy::cast_sign_loss)]
    Ok(counts.first().copied().unwrap_or(0).max(0) as usize)
}

/// Delete the `excess` oldest spans in `namespace`, oldest append time first.
///
/// The victims are selected in a first pass (ordered by append time, ties broken
/// by id for a stable cut) and deleted by their concrete record ids in a second.
/// Resolving the ids in app code rather than a nested delete-subquery keeps the
/// eviction set explicit and the cut deterministic.
async fn delete_oldest(db: &Surreal<Db>, namespace: &str, excess: usize) -> Result<()> {
    let victims: Vec<RecordId> = db
        .query(format!(
            "SELECT VALUE id FROM {TRACE_TABLE} WHERE namespace = $ns \
             ORDER BY appended ASC, id ASC LIMIT $excess"
        ))
        .bind(("ns", namespace.to_owned()))
        .bind(("excess", excess as i64))
        .await
        .map_err(TraceError::Retain)?
        .take(0)
        .map_err(TraceError::Retain)?;

    for id in victims {
        let _: Option<serde_json::Value> = db.delete(id).await.map_err(TraceError::Retain)?;
    }
    Ok(())
}
