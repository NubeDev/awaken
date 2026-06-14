//! Atomic before/after capture: run a command's mutation, take the before-image.
//!
//! Contract #1/#4 (`rubix/STACK-DEISGN.md`): the gate captures before/after
//! atomically with the write. This step executes the `RETURN BEFORE` statement
//! built by [`persist`](super::persist) against the store handle and decodes the
//! prior row state — the before-image — in the *same* round trip as the write,
//! so capturing it never costs a separate read. The after-image is the content
//! the command wrote (none for a delete). The captured change is consumed by
//! audit here and by undo in WS-06 — one capture, two consumers
//! (`rubix/docs/SCOPE.md`, "Audit and undo derive from the same captured
//! change").

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{RecordId, SurrealValue};

use rubix_core::Id;

use crate::error::{GateError, Result};

use super::action::Change;
use super::persist::{RECORD_TABLE, mutation_for};

/// The before/after pair captured atomically with a command's write.
///
/// `before` is the row state prior to the mutation (`None` when the command
/// created a row that did not exist); `after` is the content the command wrote
/// (`None` for a delete). Both are free-form JSON summaries — the audit row and
/// the WS-06 undo stack project from this one capture.
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedChange {
    /// The target record's content before the mutation, if it existed.
    pub before: Option<serde_json::Value>,
    /// The content the mutation wrote, if any.
    pub after: Option<serde_json::Value>,
}

/// Apply `change` to `target` and capture its before-image in one round trip.
///
/// Runs the `RETURN BEFORE` mutation on the store handle, binding the target
/// thing, namespace, and (for create/update) content. The returned
/// [`CapturedChange`] carries the prior row content as `before` and the written
/// content as `after`.
///
/// # Errors
/// Returns [`GateError::CommandApply`] if the mutation fails or its result
/// cannot be decoded.
pub(crate) async fn capture(
    db: &Surreal<Db>,
    namespace: &str,
    target: &Id,
    change: &Change,
) -> Result<CapturedChange> {
    let mutation = mutation_for(change);
    let thing = RecordId::new(RECORD_TABLE, target.as_str());
    let mut query = db
        .query(&mutation.statement)
        .bind(("record", thing))
        .bind(("namespace", namespace.to_owned()));
    if let Some(content) = mutation.writes.clone() {
        query = query.bind(("content", content));
    }
    let mut response = query.await.map_err(GateError::CommandApply)?;
    let before_row: Option<BeforeRow> = response.take(0).map_err(GateError::CommandApply)?;
    Ok(CapturedChange {
        before: before_row.and_then(|row| row.content),
        after: mutation.writes,
    })
}

/// The fields of the before-image the audit summary needs.
///
/// `RETURN BEFORE` yields the full prior row; only the free-form `content` is
/// summarised into the audit/undo capture, so the rest of the row is ignored.
/// `content` is optional because a create's before-image is `NONE`.
#[derive(Debug, SurrealValue)]
struct BeforeRow {
    content: Option<serde_json::Value>,
}
