//! Integration: a config conflict resolves by ownership, ahead of any LWW.
//!
//! The config plane reconciles ownership first (`rubix/docs/sessions/WS-15.md`):
//! the cloud owns shared and per-tenant config, the edge owns local-only config.
//! Where a scope names an owner the conflict is settled outright — the owner's
//! version wins even when the *other* side wrote (and was audited) strictly later,
//! so ownership is unambiguously ahead of last-write-wins. This test edits the same
//! definition on both sides and asserts ownership decides each scope.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_sync::{ConfigScope, ConfigVersion, Owner, reconcile};
use surrealdb::types::Datetime;

fn cloud_version(updated: Datetime, audit_at: Datetime) -> ConfigVersion {
    ConfigVersion::new(Owner::Cloud, serde_json::json!({ "from": "cloud" }), updated, audit_at)
}

fn edge_version(updated: Datetime, audit_at: Datetime) -> ConfigVersion {
    ConfigVersion::new(Owner::Edge, serde_json::json!({ "from": "edge" }), updated, audit_at)
}

#[test]
fn shared_config_is_won_by_the_cloud_even_when_the_edge_wrote_later() {
    let earlier = Datetime::default();
    let later = Datetime::now();
    // Cloud wrote earlier; edge wrote (and was audited) later. Ownership still wins.
    let cloud = cloud_version(earlier, earlier);
    let edge = edge_version(later, later);
    let winner = reconcile(ConfigScope::Shared, cloud, edge);
    assert_eq!(winner.side, Owner::Cloud);
    assert_eq!(winner.content, serde_json::json!({ "from": "cloud" }));
}

#[test]
fn per_tenant_config_is_won_by_the_cloud() {
    let earlier = Datetime::default();
    let later = Datetime::now();
    let cloud = cloud_version(earlier, earlier);
    let edge = edge_version(later, later);
    let winner = reconcile(ConfigScope::Tenant, cloud, edge);
    assert_eq!(winner.side, Owner::Cloud, "cloud is the multi-tenant authority");
}

#[test]
fn local_only_config_is_won_by_the_edge_even_when_the_cloud_wrote_later() {
    let earlier = Datetime::default();
    let later = Datetime::now();
    // Cloud wrote later; edge owns local-only config, so the edge still wins.
    let cloud = cloud_version(later, later);
    let edge = edge_version(earlier, earlier);
    let winner = reconcile(ConfigScope::LocalOnly, cloud, edge);
    assert_eq!(winner.side, Owner::Edge);
    assert_eq!(winner.content, serde_json::json!({ "from": "edge" }));
}
