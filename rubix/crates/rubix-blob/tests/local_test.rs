//! Integration: the local-filesystem blob store round-trips and isolates tenants.

use std::path::PathBuf;

use rubix_blob::{BlobError, BlobStore, FileRef, LocalFsBlobStore};

/// A unique temp root per test so parallel runs do not collide.
fn temp_root(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("rubix-blob-test-{tag}-{}", std::process::id()));
    dir
}

#[tokio::test]
async fn put_then_load_round_trips_bytes_and_reference() {
    let root = temp_root("roundtrip");
    let store = LocalFsBlobStore::open(&root);
    let reference = FileRef::new("blob-1", "plan.pdf", 5, "application/pdf");

    store
        .put("acme", &reference, b"hello")
        .await
        .expect("put");
    let loaded = store.load("acme", "blob-1").await.expect("load");

    assert_eq!(loaded.bytes, b"hello");
    assert_eq!(loaded.reference, reference);

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_blob_is_not_visible_from_another_namespace() {
    let root = temp_root("isolation");
    let store = LocalFsBlobStore::open(&root);
    let reference = FileRef::new("secret", "a.txt", 3, "text/plain");

    store.put("acme", &reference, b"abc").await.expect("put");

    // Same id, different namespace → not found. Tenant isolation by path.
    let other = store.load("globex", "secret").await;
    assert!(matches!(other, Err(BlobError::NotFound)));

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn loading_an_unknown_blob_is_not_found() {
    let root = temp_root("missing");
    let store = LocalFsBlobStore::open(&root);
    let result = store.load("acme", "nope").await;
    assert!(matches!(result, Err(BlobError::NotFound)));
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn delete_is_idempotent() {
    let root = temp_root("delete");
    let store = LocalFsBlobStore::open(&root);
    let reference = FileRef::new("blob-x", "x.bin", 1, "application/octet-stream");
    store.put("acme", &reference, b"x").await.expect("put");

    store.delete("acme", "blob-x").await.expect("first delete");
    // Deleting again must not error.
    store.delete("acme", "blob-x").await.expect("second delete");
    assert!(matches!(
        store.load("acme", "blob-x").await,
        Err(BlobError::NotFound)
    ));

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_path_traversing_id_is_rejected() {
    let root = temp_root("traverse");
    let store = LocalFsBlobStore::open(&root);
    let reference = FileRef::new("..", "evil", 1, "text/plain");
    let result = store.put("acme", &reference, b"x").await;
    assert!(matches!(result, Err(BlobError::InvalidId(_))));
    let _ = std::fs::remove_dir_all(&root);
}
