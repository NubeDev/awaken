//! Integration: an extension registers and authenticates as a scoped
//! service-account principal.
//!
//! Exercises the WS-13 identity model end to end against a live kv-mem
//! SurrealDB: `register_extension` provisions an `Extension`-kind principal on
//! the same identity path a user uses, the extension then authenticates with its
//! subject/secret token and resolves to that very principal, and signing in
//! yields a namespace-scoped session — the same two checks a user passes. A wrong
//! secret is rejected, proving the credential is real, not assumed.

#[path = "../ext/mod.rs"]
mod ext;

use rubix_core::PrincipalKind;
use rubix_gate::{PrincipalToken, authenticate, issue_scoped_session};

use ext::open::{NS, open_ext_store};

#[tokio::test]
async fn an_extension_authenticates_as_a_scoped_service_account_principal() {
    let handle = open_ext_store("ext_register").await;

    let registration =
        rubix_ext::register_extension(handle.raw(), "weather-ext", NS, "s3cret")
            .await
            .expect("register extension");

    // It is an extension-kind principal bound to the namespace.
    assert_eq!(registration.principal().kind, PrincipalKind::Extension);
    assert_eq!(registration.principal().namespace, NS);

    // It authenticates on the same path as a user and resolves to that identity.
    let token = PrincipalToken::new("weather-ext", "s3cret");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate extension");
    assert_eq!(&resolved, registration.principal());

    // And it can sign in to a namespace-scoped session like any principal.
    let session = issue_scoped_session(handle.raw(), NS, "ext_register", resolved, &token)
        .await
        .expect("issue scoped session for extension");
    assert_eq!(session.principal().kind, PrincipalKind::Extension);
}

#[tokio::test]
async fn a_wrong_secret_is_rejected() {
    let handle = open_ext_store("ext_register_badsecret").await;
    rubix_ext::register_extension(handle.raw(), "weather-ext", NS, "s3cret")
        .await
        .expect("register extension");

    let token = PrincipalToken::new("weather-ext", "wrong");
    let err = authenticate(handle.raw(), &token)
        .await
        .expect_err("a wrong secret must be rejected");
    assert!(matches!(err, rubix_gate::GateError::Authenticate(_)));
}
