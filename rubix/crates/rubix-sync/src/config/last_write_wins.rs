//! Last-write-wins resolution between two versions of the same config.
//!
//! Where ownership does not settle a config conflict
//! ([`own`](super::own)) — an overlap that is unavoidable — reconciliation falls
//! back to last-write-wins (`rubix/docs/SCOPE.md`, "Sync and conflict model"): the
//! version with the later write instant wins. A true tie on the write instant is
//! not decided here; it is handed to [`tiebreak`](super::tiebreak), which breaks
//! it on the WS-05 audit timestamp. Keeping the tie *undecided* at this layer is
//! deliberate — LWW must never silently pick a side on equal timestamps, which is
//! exactly the case a tiebreak exists for.

use std::cmp::Ordering;

use surrealdb::types::Datetime;

use super::own::Owner;

/// One side's version of a contested config definition.
///
/// Carries the side it came from, its content, the instant it was last written
/// (`updated`, the LWW key), and the instant its write was audited (`audit_at`,
/// the WS-05 tiebreak). Both timestamps are needed up front so a reconcile never
/// has to go back to the store mid-decision.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigVersion {
    /// Which side produced this version.
    pub side: Owner,
    /// The config content this version holds.
    pub content: serde_json::Value,
    /// When this version was last written — the last-write-wins key.
    pub updated: Datetime,
    /// When this version's write was audited — the tie tiebreak (WS-05).
    pub audit_at: Datetime,
}

impl ConfigVersion {
    /// Build a config version.
    #[must_use]
    pub fn new(
        side: Owner,
        content: serde_json::Value,
        updated: Datetime,
        audit_at: Datetime,
    ) -> Self {
        Self {
            side,
            content,
            updated,
            audit_at,
        }
    }
}

/// The outcome of comparing two versions by their write instant.
///
/// A decisive winner ([`Winner`](LwwOutcome::Winner)) when the write instants
/// differ; [`Tie`](LwwOutcome::Tie) when they are equal, deferring the decision to
/// the audit-timestamp tiebreak.
#[derive(Debug, Clone, PartialEq)]
pub enum LwwOutcome {
    /// One version was written strictly later — it wins.
    Winner(ConfigVersion),
    /// Both were written at the same instant; the audit timestamp must break it.
    Tie,
}

/// Resolve two versions of the same config by last-write-wins.
///
/// The version with the later `updated` instant wins. Equal instants are a
/// [`Tie`](LwwOutcome::Tie) — not resolved here, see
/// [`tiebreak`](super::tiebreak::break_tie).
#[must_use]
pub fn last_write_wins(a: ConfigVersion, b: ConfigVersion) -> LwwOutcome {
    match a.updated.cmp(&b.updated) {
        Ordering::Greater => LwwOutcome::Winner(a),
        Ordering::Less => LwwOutcome::Winner(b),
        Ordering::Equal => LwwOutcome::Tie,
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigVersion, LwwOutcome, last_write_wins};
    use crate::config::own::Owner;
    use surrealdb::types::Datetime;

    fn version(side: Owner, updated: Datetime) -> ConfigVersion {
        ConfigVersion::new(side, serde_json::json!({ "v": 1 }), updated, updated)
    }

    #[test]
    fn the_later_write_wins() {
        let early = version(Owner::Edge, Datetime::default());
        let late = version(Owner::Cloud, Datetime::now());
        match last_write_wins(early, late.clone()) {
            LwwOutcome::Winner(w) => assert_eq!(w, late),
            LwwOutcome::Tie => panic!("distinct instants must not tie"),
        }
    }

    #[test]
    fn equal_write_instants_are_a_tie() {
        let at = Datetime::now();
        let a = version(Owner::Edge, at);
        let b = version(Owner::Cloud, at);
        assert_eq!(last_write_wins(a, b), LwwOutcome::Tie);
    }
}
