//! Integration: k-nearest search returns vectors nearest a probe, scoped.
//!
//! Proves the vector / semantic-search surface (`rubix/docs/sessions/WS-09.md`):
//! vectors live beside the records (`rubix/docs/SCOPE.md`, principle 6), and a
//! k-nearest query over a record's vector column returns the closest `k` in
//! ascending-distance order — run on SurrealDB (SurrealQL first, contract #6)
//! through the principal's scoped session, so only readable records can match.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::{Id, Record, Role, create_record};
use rubix_query::nearest;

use fixture::open::{open_query_store, scoped_session_for};

/// Seed a record whose `content.embedding` is `vector`.
async fn seed_vector(handle: &rubix_store::StoreHandle, vector: &[f64]) -> Id {
    let record = Record::new("rubix", serde_json::json!({ "embedding": vector }));
    let id = record.id.clone();
    create_record(handle.raw(), &record).await.expect("seed");
    id
}

#[tokio::test]
async fn nearest_returns_neighbours_in_ascending_distance() {
    let database = "nearest_order";
    let handle = open_query_store(database).await;

    // Three 2-D vectors at increasing distance from the probe [0, 0].
    let near = seed_vector(&handle, &[0.1, 0.0]).await;
    let mid = seed_vector(&handle, &[1.0, 0.0]).await;
    let far = seed_vector(&handle, &[5.0, 0.0]).await;

    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    let hits = nearest(
        session.connection(),
        "record",
        "content.embedding",
        &[0.0, 0.0],
        3,
    )
    .await
    .expect("nearest");

    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].id, near.to_string());
    assert_eq!(hits[1].id, mid.to_string());
    assert_eq!(hits[2].id, far.to_string());
    assert!(hits[0].distance <= hits[1].distance);
    assert!(hits[1].distance <= hits[2].distance);
}

#[tokio::test]
async fn k_limits_the_number_of_neighbours() {
    let database = "nearest_k";
    let handle = open_query_store(database).await;
    for offset in 1..=5 {
        seed_vector(&handle, &[f64::from(offset), 0.0]).await;
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "bob", "rubix", Role::Viewer).await;

    let hits = nearest(
        session.connection(),
        "record",
        "content.embedding",
        &[0.0, 0.0],
        2,
    )
    .await
    .expect("nearest");
    assert_eq!(hits.len(), 2, "k bounds the result count");
}

#[tokio::test]
async fn a_zero_k_or_empty_probe_returns_no_hits() {
    let database = "nearest_empty";
    let handle = open_query_store(database).await;
    seed_vector(&handle, &[1.0, 0.0]).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "carol", "rubix", Role::Viewer).await;

    let none = nearest(session.connection(), "record", "content.embedding", &[0.0], 0)
        .await
        .expect("zero k");
    assert!(none.is_empty());

    let empty_probe = nearest(session.connection(), "record", "content.embedding", &[], 3)
        .await
        .expect("empty probe");
    assert!(empty_probe.is_empty());
}

#[tokio::test]
async fn an_injected_field_path_is_rejected() {
    let database = "nearest_inject";
    let handle = open_query_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "dave", "rubix", Role::Viewer).await;

    let err = nearest(
        session.connection(),
        "record",
        "embedding; DROP TABLE record",
        &[0.0],
        1,
    )
    .await
    .expect_err("an injected field path must be rejected");
    assert!(err.to_string().contains("field path"), "{err}");
}
