//! Integration: the opaque login-token lifecycle against a live kv-mem SurrealDB.
//!
//! Exercises the session-issuance surface (OQ10): a minted token resolves back to
//! the credentials it was issued for, scoped to its namespace; a revoked token no
//! longer resolves; an unknown token resolves to nothing; and the raw token is
//! never stored in the clear (only its hash).

#[path = "../gate/mod.rs"]
mod gate;

use rubix_gate::{
    PrincipalToken, issue_session_token, resolve_session_token, revoke_session_token,
};

use gate::open::{NS, open_gate_store};

const DB: &str = "main";

#[tokio::test]
async fn a_minted_token_resolves_to_its_credentials_and_scope() {
    let handle = open_gate_store("token_resolve").await;
    let creds = PrincipalToken::new("alice", "s3cret");

    let issued = issue_session_token(handle.raw(), &creds, NS, DB, 3600)
        .await
        .expect("issue");
    assert!(!issued.value.is_empty());
    assert!(!issued.expires.is_empty());

    let resolved = resolve_session_token(handle.raw(), &issued.value)
        .await
        .expect("resolve")
        .expect("present");
    assert_eq!(resolved.token, creds);
    assert_eq!(resolved.namespace, NS);
    assert_eq!(resolved.database, DB);
}

#[tokio::test]
async fn an_unknown_token_resolves_to_none() {
    let handle = open_gate_store("token_unknown").await;
    let resolved = resolve_session_token(handle.raw(), "not-a-real-token")
        .await
        .expect("resolve");
    assert!(resolved.is_none());
}

#[tokio::test]
async fn a_revoked_token_no_longer_resolves() {
    let handle = open_gate_store("token_revoke").await;
    let creds = PrincipalToken::new("bob", "pw");
    let issued = issue_session_token(handle.raw(), &creds, NS, DB, 3600)
        .await
        .expect("issue");

    assert!(revoke_session_token(handle.raw(), &issued.value)
        .await
        .expect("revoke"));
    assert!(resolve_session_token(handle.raw(), &issued.value)
        .await
        .expect("resolve")
        .is_none());
    // A second revoke is a no-op, not an error.
    assert!(!revoke_session_token(handle.raw(), &issued.value)
        .await
        .expect("revoke again"));
}

#[tokio::test]
async fn an_expired_token_does_not_resolve() {
    let handle = open_gate_store("token_expired").await;
    let creds = PrincipalToken::new("carol", "pw");
    let issued = issue_session_token(handle.raw(), &creds, NS, DB, 3600)
        .await
        .expect("issue");

    // Backdate the token's expiry into the past, then assert it no longer
    // resolves — the resolve query requires `expires > time::now()`.
    handle
        .raw()
        .query("UPDATE session_token SET expires = time::now() - 1h")
        .await
        .expect("backdate expiry")
        .check()
        .expect("update applied");

    assert!(resolve_session_token(handle.raw(), &issued.value)
        .await
        .expect("resolve")
        .is_none());
}

#[tokio::test]
async fn the_raw_token_is_never_stored_in_the_clear() {
    let handle = open_gate_store("token_hashed").await;
    let creds = PrincipalToken::new("dan", "pw");
    let issued = issue_session_token(handle.raw(), &creds, NS, DB, 3600)
        .await
        .expect("issue");

    // No stored row carries the raw token value anywhere.
    let mut response = handle
        .raw()
        .query("SELECT VALUE token_hash FROM session_token")
        .await
        .expect("select hashes");
    let hashes: Vec<String> = response.take(0).expect("decode");
    assert_eq!(hashes.len(), 1);
    assert_ne!(hashes[0], issued.value, "token must be hashed at rest");
    assert!(!hashes[0].is_empty());
}
