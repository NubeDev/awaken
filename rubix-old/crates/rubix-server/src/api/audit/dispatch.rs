//! Shared helpers for the undo/redo dispatch (docs/design/audit-and-undo.md
//! "Undo/Redo"): the per-actor cursor key and the touched-id projection the UI uses
//! to invalidate exactly the affected queries.

use rubix_core::{Actor, Change};
use uuid::Uuid;

/// The cursor key for an actor. A human principal pops/advances its own subject's
/// stack; agent and system edits collapse onto stable keys (see
/// [`Actor::cursor_subject`]).
pub(crate) fn cursor_subject(actor: &Actor) -> String {
    actor.cursor_subject().to_string()
}

/// The distinct resource ids a change group touched, preserving first-seen order so
/// the UI can invalidate exactly those query keys.
pub(crate) fn touched_ids(changes: &[Change]) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(changes.len());
    for change in changes {
        if !ids.contains(&change.resource_id) {
            ids.push(change.resource_id);
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rubix_core::Change;

    #[test]
    fn touched_ids_are_distinct_and_ordered() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let g = Uuid::new_v4();
        let mk = |rid: Uuid| {
            Change::create(
                Uuid::new_v4(),
                Utc::now(),
                "kfc",
                None,
                Actor::System,
                "dashboard",
                rid,
                serde_json::json!({}),
                g,
                None,
            )
        };
        let rows = vec![mk(a), mk(b), mk(a)];
        assert_eq!(touched_ids(&rows), vec![a, b]);
    }

    #[test]
    fn cursor_subject_maps_actor_variants() {
        assert_eq!(
            cursor_subject(&Actor::User { subject: "sub-1".into() }),
            "sub-1"
        );
        assert_eq!(cursor_subject(&Actor::System), "@system");
    }
}
