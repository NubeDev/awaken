//! Derive the edge-identity partition key for an ingested stream.
//!
//! Ingested data is written append-only into a partition keyed by the edge
//! identity (`rubix/STACK-DEISGN.md`, contract #5: "two edges never write the
//! same records, so reconciliation is ordering + dedup, not merge"). The edge
//! identity is the principal's namespace — the same tenant the SurrealDB
//! row-level read scope and every other plane partition on (`rubix-core`
//! `Record::namespace`, `rubix-trace` span rows). This file is the one place the
//! partition key is derived from the principal, so the Zenoh key-space root
//! (what a principal may subscribe to) and the persisted record namespace (where
//! its samples land) cannot drift apart.

use rubix_core::Principal;

/// The Zenoh key-space root every ingest key-expression lives under.
///
/// A principal subscribes within `<INGEST_ROOT>/<namespace>/…`; the leading
/// segment namespaces the platform's own ingest traffic away from any other
/// Zenoh users sharing the fabric.
pub const INGEST_ROOT: &str = "rubix/ingest";

/// The partition key an ingested record is written under for `principal`.
///
/// Returns the principal's namespace — the edge identity. Two edges have
/// distinct namespaces, so their append-only writes never collide (contract #5).
#[must_use]
pub fn partition_for(principal: &Principal) -> &str {
    &principal.namespace
}

/// The Zenoh key-space subtree `principal` is confined to.
///
/// Every key expression a principal may subscribe to is included in this subtree,
/// so the subscribe scope and the persistence partition share one edge-identity
/// root. The returned expression is `<INGEST_ROOT>/<namespace>/**` — a wildcard
/// subtree so that `authorize` can require the requested scope to be *included*
/// in it (Zenoh inclusion: `a/b/**` includes `a/b/**`, `a/b/x`, `a/b/x/y`, but
/// not a sibling edge's `a/c/**`).
#[must_use]
pub fn keyspace_root(principal: &Principal) -> String {
    format!("{INGEST_ROOT}/{}/**", principal.namespace)
}

#[cfg(test)]
mod tests {
    use super::{INGEST_ROOT, keyspace_root, partition_for};
    use rubix_core::{Id, Principal, PrincipalKind, Role};

    fn principal(namespace: &str) -> Principal {
        Principal::new(Id::from_raw("p-1"), namespace, PrincipalKind::User, Role::Operator)
    }

    #[test]
    fn partition_key_is_the_principal_namespace() {
        assert_eq!(partition_for(&principal("edge-7")), "edge-7");
    }

    #[test]
    fn keyspace_root_nests_the_namespace_subtree_under_the_ingest_root() {
        assert_eq!(keyspace_root(&principal("edge-7")), format!("{INGEST_ROOT}/edge-7/**"));
    }

    #[test]
    fn distinct_edges_get_distinct_partitions() {
        assert_ne!(partition_for(&principal("edge-a")), partition_for(&principal("edge-b")));
    }
}
