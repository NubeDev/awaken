//! Integration: the key-space is one capability decision, taken at subscribe.
//!
//! The permitted Zenoh key-space is resolved once via the WS-04 gate, never per
//! message (`rubix/docs/sessions/WS-12.md`, contract #2). This test exercises the
//! three outcomes of that one decision: a granted principal requesting a key-space
//! inside its edge partition is authorized; an ungranted principal is refused
//! (fail closed); and a granted principal requesting a key-space outside its
//! partition is refused — a principal cannot subscribe into another edge's data.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_ingest::authorize_keyspace;

use fixture::open::{granted_principal, open_ingest_store, ungranted_principal};

#[tokio::test]
async fn a_granted_in_partition_keyspace_is_authorized() {
    let handle = open_ingest_store("auth_ok").await;
    let principal = granted_principal(&handle, "ingestor").await;

    let authorized = authorize_keyspace(handle.raw(), &principal, "rubix/ingest/edge-7/**")
        .await
        .expect("authorize in-partition key-space");
    assert!(authorized.scope().as_str().starts_with("rubix/ingest/edge-7"));
}

#[tokio::test]
async fn an_ungranted_principal_is_refused_at_subscribe() {
    let handle = open_ingest_store("auth_ungranted").await;
    let principal = ungranted_principal(&handle, "ingestor").await;

    let result = authorize_keyspace(handle.raw(), &principal, "rubix/ingest/edge-7/**").await;
    assert!(result.is_err(), "ungranted subscribe must fail closed");
}

#[tokio::test]
async fn an_out_of_partition_keyspace_is_refused() {
    let handle = open_ingest_store("auth_out_of_partition").await;
    // Granted the capability, but requesting another edge's key-space.
    let principal = granted_principal(&handle, "ingestor").await;

    let result = authorize_keyspace(handle.raw(), &principal, "rubix/ingest/edge-99/**").await;
    assert!(result.is_err(), "a key-space outside the edge partition must be refused");
}

#[tokio::test]
async fn a_malformed_keyspace_is_rejected() {
    let handle = open_ingest_store("auth_malformed").await;
    let principal = granted_principal(&handle, "ingestor").await;

    // An empty key expression is not a valid Zenoh scope.
    let result = authorize_keyspace(handle.raw(), &principal, "").await;
    assert!(result.is_err(), "a malformed key-space must be rejected");
}
