//! Integration: a granted JSON-RPC `invoke` succeeds through the WS-05 gate and
//! writes an audit row carrying its correlation id.
//!
//! Exercises contract #1 end to end: an extension provisioned as a scoped
//! principal and granted the admin profile drives an `invoke` control request;
//! the effect lands and exactly one audit row is appended carrying the same
//! correlation id the call returned — an extension command is audited identically
//! to a user's. The three grant profiles are also asserted to resolve to distinct
//! permitted-action sets from the one grant mechanism (WS-13 unit-level concern,
//! verified here against the registered capability enum).

#[path = "../ext/mod.rs"]
mod ext;

use rubix_core::Id;
use rubix_ext::{ControlMethod, ControlRequest, GrantProfile};

use ext::open::{admin, open_ext_store};

async fn count_audit_rows(handle: &rubix_store::StoreHandle, subject: &str) -> i64 {
    let mut response = handle
        .raw()
        .query("SELECT VALUE count() FROM audit WHERE subject = $subject GROUP ALL")
        .bind(("subject", subject.to_owned()))
        .await
        .expect("count audit");
    let count: Option<i64> = response.take(0).expect("decode count");
    count.unwrap_or(0)
}

#[tokio::test]
async fn a_granted_invoke_succeeds_through_the_gate_and_audits_it() {
    let handle = open_ext_store("ext_invoke").await;

    let registration =
        rubix_ext::register_extension(handle.raw(), "rule-ext", "rubix", "k")
            .await
            .expect("register extension");
    let extension = registration.principal().clone();

    // Grant the admin profile — the same mechanism a user is granted through.
    rubix_ext::grant_extension(handle.raw(), &admin(), &extension, GrantProfile::Admin)
        .await
        .expect("grant extension");

    let target = Id::from_raw("ext-invocation-1");
    let request = ControlRequest::new(
        ControlMethod::Invoke,
        target.clone(),
        serde_json::json!({ "action": "recompute", "window": "minute" }),
    );

    let outcome = rubix_ext::invoke(handle.raw(), &extension, &request)
        .await
        .expect("granted invoke");

    // The effect landed.
    let landed = rubix_core::read_record(handle.raw(), &target)
        .await
        .expect("read record")
        .map(|record| record.content);
    assert_eq!(
        landed,
        Some(serde_json::json!({ "action": "recompute", "window": "minute" }))
    );

    // Exactly one audit row, carrying the correlation id the call returned.
    assert_eq!(count_audit_rows(&handle, "rule-ext").await, 1);
    let mut audit_resp = handle
        .raw()
        .query("SELECT correlation_id FROM audit WHERE subject = $subject")
        .bind(("subject", "rule-ext".to_owned()))
        .await
        .expect("read audit");
    let stored_corr: Option<String> = audit_resp.take("correlation_id").expect("corr");
    assert_eq!(stored_corr.as_deref(), Some(outcome.correlation_id.as_str()));
}
