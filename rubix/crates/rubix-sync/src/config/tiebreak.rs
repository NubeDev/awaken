//! Break a last-write-wins tie on the WS-05 audit timestamp.
//!
//! When two versions of the same config carry the identical write instant,
//! last-write-wins ([`last_write_wins`](super::last_write_wins)) cannot decide and
//! returns a tie. The tiebreak is the WS-05 audit log timestamp
//! (`rubix/docs/SCOPE.md`, "Sync and conflict model": LWW with the audit log as
//! tiebreak): the version whose write was audited later wins. The audit timestamp
//! is the authoritative record of *when* a change was committed through the gate,
//! so it is the right total order to fall back on — and because the audit log is
//! append-only and immutable (WS-05), it cannot be gamed after the fact.
//!
//! Should even the audit timestamps tie (the same instant to the resolution of
//! the clock), the deterministic final fallback is the owning side: cloud over
//! edge, so two receivers always converge to the same winner rather than
//! oscillating.

use std::cmp::Ordering;

use super::last_write_wins::ConfigVersion;
use super::own::Owner;

/// Choose the winner of an LWW tie by the audit timestamp.
///
/// The version with the later `audit_at` wins. If the audit timestamps are also
/// equal, the cloud side wins as the deterministic final fallback so the decision
/// is stable across receivers.
#[must_use]
pub fn break_tie(a: ConfigVersion, b: ConfigVersion) -> ConfigVersion {
    match a.audit_at.cmp(&b.audit_at) {
        Ordering::Greater => a,
        Ordering::Less => b,
        Ordering::Equal => deterministic_fallback(a, b),
    }
}

/// The final, deterministic fallback when even the audit timestamps tie.
///
/// Cloud wins over edge — an arbitrary but fixed total order, so two receivers
/// presented with the same pair always pick the same side.
fn deterministic_fallback(a: ConfigVersion, b: ConfigVersion) -> ConfigVersion {
    match (a.side, b.side) {
        (Owner::Cloud, _) => a,
        (_, Owner::Cloud) => b,
        // Both edge (no cloud side present): keep `a` deterministically.
        _ => a,
    }
}

#[cfg(test)]
mod tests {
    use super::break_tie;
    use crate::config::last_write_wins::ConfigVersion;
    use crate::config::own::Owner;
    use surrealdb::types::Datetime;

    fn version(side: Owner, audit_at: Datetime) -> ConfigVersion {
        // Same `updated` for both — this layer only ever sees a tied write instant.
        ConfigVersion::new(side, serde_json::json!({ "side": "x" }), Datetime::default(), audit_at)
    }

    #[test]
    fn the_later_audited_write_wins_the_tie() {
        let earlier = version(Owner::Edge, Datetime::default());
        let later = version(Owner::Cloud, Datetime::now());
        assert_eq!(break_tie(earlier, later.clone()), later);
    }

    #[test]
    fn equal_audit_timestamps_fall_back_to_cloud() {
        let at = Datetime::now();
        let cloud = version(Owner::Cloud, at);
        let edge = version(Owner::Edge, at);
        // Cloud wins regardless of argument order, so the decision is stable.
        assert_eq!(break_tie(edge.clone(), cloud.clone()).side, Owner::Cloud);
        assert_eq!(break_tie(cloud, edge).side, Owner::Cloud);
    }
}
