//! Integration: where ownership is ambiguous, LWW + the audit-timestamp tiebreak.
//!
//! Ownership settles most config conflicts, but where overlap is unavoidable —
//! neither side is the declared owner — reconciliation falls back to last-write-
//! wins (`rubix/docs/sessions/WS-15.md`): the later write wins, and a true
//! write-instant tie is broken on the WS-05 audit timestamp. This test asserts the
//! later write wins on an ambiguous overlap, and that an exact write-instant tie
//! falls through to the audit timestamp as the tiebreak.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_sync::{ConfigVersion, Owner, reconcile_ambiguous};
use surrealdb::types::Datetime;

fn version(side: Owner, updated: Datetime, audit_at: Datetime) -> ConfigVersion {
    let tag = match side {
        Owner::Cloud => "cloud",
        Owner::Edge => "edge",
    };
    ConfigVersion::new(side, serde_json::json!({ "from": tag }), updated, audit_at)
}

#[test]
fn the_later_write_wins_an_ambiguous_overlap() {
    let earlier = Datetime::default();
    let later = Datetime::now();
    let cloud = version(Owner::Cloud, earlier, earlier);
    let edge = version(Owner::Edge, later, later);
    // No ownership to invoke: the strictly later write (edge) wins on LWW.
    let winner = reconcile_ambiguous(cloud, edge);
    assert_eq!(winner.side, Owner::Edge);
}

#[test]
fn an_exact_write_tie_is_broken_on_the_audit_timestamp() {
    let written_at = Datetime::now();
    let audited_early = Datetime::default();
    let audited_late = Datetime::now();
    // Identical write instant; the cloud version was audited later, so it wins the
    // tiebreak even though the edge version is the second argument.
    let cloud = version(Owner::Cloud, written_at, audited_late);
    let edge = version(Owner::Edge, written_at, audited_early);
    let winner = reconcile_ambiguous(cloud, edge);
    assert_eq!(
        winner.side,
        Owner::Cloud,
        "the later-audited write breaks a write-instant tie",
    );
}

#[test]
fn a_total_tie_resolves_deterministically_to_cloud() {
    let at = Datetime::now();
    // Same write instant and same audit instant: the deterministic final fallback
    // is cloud, so two receivers converge rather than oscillate.
    let cloud = version(Owner::Cloud, at, at);
    let edge = version(Owner::Edge, at, at);
    assert_eq!(reconcile_ambiguous(cloud.clone(), edge.clone()).side, Owner::Cloud);
    assert_eq!(reconcile_ambiguous(edge, cloud).side, Owner::Cloud);
}
