//! The reversible change record — a forward change plus its inverse.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Undo/redo"):
//! audit and undo derive from the **same** captured change — audit takes the
//! immutable projection, undo takes the mutable stack. This file owns the undo
//! side: it builds a [`ChangeRecord`] from the [`CapturedChange`] the gate took
//! atomically with the write (WS-05), pairing the forward [`Change`] with the
//! inverse [`Change`] that restores the prior value. No new read is needed — the
//! before-image is already in hand, so the inverse is cheap (the document
//! before/after snapshot the SCOPE doc calls "cheap via before/after").

use crate::command::{CapturedChange, Change};

/// A definition mutation paired with the change that reverses it.
///
/// `forward` is the change the gate applied; `inverse` is the change `undo`
/// re-enters the gate with to restore the prior value. The inverse is derived
/// from the captured before-image: a create reverses to a delete, a delete
/// reverses to a create of the prior content, and an update reverses to an
/// update back to the prior content.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeRecord {
    /// The change the command applied.
    pub forward: Change,
    /// The change that reverses [`forward`](ChangeRecord::forward).
    pub inverse: Change,
}

impl ChangeRecord {
    /// Build the reversible record from a forward change and its capture.
    ///
    /// The inverse is derived from `capture.before` (the prior content taken
    /// atomically with the write): a create with no prior row reverses to a
    /// delete; a delete or update of an existing row reverses by writing the
    /// prior content back. When a create somehow observed a prior row, the
    /// inverse still restores that prior content rather than deleting it, so the
    /// reversal never destroys pre-existing state.
    #[must_use]
    pub fn from_capture(forward: &Change, capture: &CapturedChange) -> Self {
        let inverse = match (forward, &capture.before) {
            (Change::Create(_), None) => Change::Delete,
            (Change::Create(_), Some(prior)) | (Change::Update(_), Some(prior)) => {
                Change::Update(prior.clone())
            }
            (Change::Delete, Some(prior)) => Change::Create(prior.clone()),
            // An update or delete with no captured prior content means the row
            // did not exist before the write, so the reversal is a delete.
            (Change::Update(_) | Change::Delete, None) => Change::Delete,
        };
        Self {
            forward: forward.clone(),
            inverse,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{CapturedChange, Change};

    use super::ChangeRecord;

    fn capture(before: Option<serde_json::Value>, after: Option<serde_json::Value>) -> CapturedChange {
        CapturedChange { before, after }
    }

    #[test]
    fn a_create_reverses_to_a_delete() {
        let forward = Change::Create(serde_json::json!({ "title": "panel" }));
        let record = ChangeRecord::from_capture(
            &forward,
            &capture(None, Some(serde_json::json!({ "title": "panel" }))),
        );
        assert_eq!(record.inverse, Change::Delete);
    }

    #[test]
    fn an_update_reverses_to_the_prior_content() {
        let prior = serde_json::json!({ "title": "old" });
        let forward = Change::Update(serde_json::json!({ "title": "new" }));
        let record = ChangeRecord::from_capture(
            &forward,
            &capture(Some(prior.clone()), Some(serde_json::json!({ "title": "new" }))),
        );
        assert_eq!(record.inverse, Change::Update(prior));
    }

    #[test]
    fn a_delete_reverses_to_a_create_of_the_prior_content() {
        let prior = serde_json::json!({ "title": "kept" });
        let record = ChangeRecord::from_capture(&Change::Delete, &capture(Some(prior.clone()), None));
        assert_eq!(record.inverse, Change::Create(prior));
    }

    #[test]
    fn forward_is_preserved_for_redo() {
        let forward = Change::Update(serde_json::json!({ "v": 2 }));
        let record = ChangeRecord::from_capture(
            &forward,
            &capture(Some(serde_json::json!({ "v": 1 })), Some(serde_json::json!({ "v": 2 }))),
        );
        assert_eq!(record.forward, forward);
    }
}
