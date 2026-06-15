//! The write events a hook can fire on.
//!
//! A hook binds to one or more record-change events
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Server-side hooks"). The set is
//! the create/update/delete the gate already audits, so a hook's `on` list reuses
//! the same vocabulary as [`Change`](crate) actions — the wire strings match the
//! audited action verbs so a hook reads the way the audit log does.

/// A record-change event a hook may fire on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    /// A record was created.
    Create,
    /// A record's content was updated.
    Update,
    /// A record was deleted.
    Delete,
}

impl HookEvent {
    /// The stable wire string for this event (matches the audited action verb).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            HookEvent::Create => "create",
            HookEvent::Update => "update",
            HookEvent::Delete => "delete",
        }
    }

    /// Resolve a stored/wire string to a known event, or `None` if unrecognised.
    #[must_use]
    pub fn parse(raw: &str) -> Option<HookEvent> {
        match raw {
            "create" => Some(HookEvent::Create),
            "update" => Some(HookEvent::Update),
            "delete" => Some(HookEvent::Delete),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HookEvent;

    #[test]
    fn every_event_round_trips_through_its_string() {
        for event in [HookEvent::Create, HookEvent::Update, HookEvent::Delete] {
            assert_eq!(HookEvent::parse(event.as_str()), Some(event));
        }
    }

    #[test]
    fn an_unknown_event_resolves_to_none() {
        assert_eq!(HookEvent::parse("upsert"), None);
    }
}
