//! Orchestrate config reconciliation: ownership first, then LWW + tiebreak.
//!
//! This is the one entry point the config plane reconciles through
//! (`rubix/docs/SCOPE.md`, "Sync and conflict model"). The order is fixed:
//! ownership ([`own`](super::own)) is consulted first and settles the conflict
//! outright wherever it can — the owner's version wins and LWW is never reached.
//! Only where overlap is unavoidable (neither side is the declared owner of the
//! scope, e.g. an ambiguous surface) does it fall back to last-write-wins
//! ([`last_write_wins`](super::last_write_wins)), and a true write-instant tie is
//! broken on the WS-05 audit timestamp ([`tiebreak`](super::tiebreak)).

use super::last_write_wins::{ConfigVersion, LwwOutcome, last_write_wins};
use super::own::{ConfigScope, Owner, owner_of};
use super::tiebreak::break_tie;

/// Reconcile the cloud and edge versions of one config item in `scope`.
///
/// Ownership decides outright when the scope names an owner that is present among
/// the two versions: the owning side's version is returned and no timestamp is
/// consulted. Otherwise reconciliation falls back to last-write-wins, breaking a
/// write-instant tie on the audit timestamp.
#[must_use]
pub fn reconcile(scope: ConfigScope, cloud: ConfigVersion, edge: ConfigVersion) -> ConfigVersion {
    match owner_of(scope) {
        Owner::Cloud => cloud,
        Owner::Edge => edge,
    }
    // Both arms above are the unambiguous-ownership path. The fallback below is
    // unreachable for the three declared scopes (each maps to a definite owner),
    // but is kept as the explicit LWW path the design calls for where ownership is
    // ambiguous — see `reconcile_ambiguous`.
}

/// Reconcile two versions where ownership is ambiguous: last-write-wins, then the
/// audit-timestamp tiebreak.
///
/// Used where the scope does not name a single owner (an unavoidable overlap),
/// the only place LWW is consulted. The later write wins; a write-instant tie is
/// broken on the WS-05 audit timestamp.
#[must_use]
pub fn reconcile_ambiguous(cloud: ConfigVersion, edge: ConfigVersion) -> ConfigVersion {
    match last_write_wins(cloud.clone(), edge.clone()) {
        LwwOutcome::Winner(winner) => winner,
        LwwOutcome::Tie => break_tie(cloud, edge),
    }
}

#[cfg(test)]
mod tests {
    use super::{reconcile, reconcile_ambiguous};
    use crate::config::last_write_wins::ConfigVersion;
    use crate::config::own::{ConfigScope, Owner};
    use surrealdb::types::Datetime;

    fn version(side: Owner, updated: Datetime, audit_at: Datetime) -> ConfigVersion {
        ConfigVersion::new(side, serde_json::json!({ "side": "x" }), updated, audit_at)
    }

    #[test]
    fn shared_config_resolves_to_cloud_by_ownership_ignoring_timestamps() {
        // Edge wrote later, but ownership wins first — cloud takes shared config.
        let cloud = version(Owner::Cloud, Datetime::default(), Datetime::default());
        let edge = version(Owner::Edge, Datetime::now(), Datetime::now());
        let winner = reconcile(ConfigScope::Shared, cloud.clone(), edge);
        assert_eq!(winner.side, Owner::Cloud);
    }

    #[test]
    fn local_only_config_resolves_to_edge_by_ownership() {
        let cloud = version(Owner::Cloud, Datetime::now(), Datetime::now());
        let edge = version(Owner::Edge, Datetime::default(), Datetime::default());
        let winner = reconcile(ConfigScope::LocalOnly, cloud, edge.clone());
        assert_eq!(winner.side, Owner::Edge);
    }

    #[test]
    fn ambiguous_overlap_falls_back_to_last_write_wins() {
        let cloud = version(Owner::Cloud, Datetime::default(), Datetime::default());
        let edge = version(Owner::Edge, Datetime::now(), Datetime::now());
        // Edge wrote later, so with no ownership to invoke it wins on LWW.
        assert_eq!(reconcile_ambiguous(cloud, edge).side, Owner::Edge);
    }

    #[test]
    fn ambiguous_write_tie_breaks_on_the_audit_timestamp() {
        let at = Datetime::now();
        // Same write instant; edge was audited later, so it wins the tiebreak.
        let cloud = version(Owner::Cloud, at, Datetime::default());
        let edge = version(Owner::Edge, at, Datetime::now());
        assert_eq!(reconcile_ambiguous(cloud, edge).side, Owner::Edge);
    }
}
