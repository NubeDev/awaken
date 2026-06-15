//! Integration: an out-of-grant JSON-RPC `invoke` is denied before any effect.
//!
//! Exercises contract #2 (the second authz layer) end to end: an extension
//! registered but holding only the read-only profile — i.e. *no* cross-plane
//! grant — drives an `invoke`. The call is refused at the capability check before
//! a command is built, so no record lands and no audit row is written. A
//! read-only extension can be a real principal yet do nothing across planes,
//! purely by the grants it lacks.

#[path = "../ext/mod.rs"]
mod ext;

use rubix_core::Id;
use rubix_ext::{ControlMethod, ControlRequest, ExtError, GrantProfile};

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
async fn an_out_of_grant_invoke_writes_nothing() {
    let handle = open_ext_store("ext_deny").await;

    let registration =
        rubix_ext::register_extension(handle.raw(), "readonly-ext", "rubix", "k")
            .await
            .expect("register extension");
    let extension = registration.principal().clone();

    // The read-only profile confers no cross-plane capability at all.
    let conferred =
        rubix_ext::grant_extension(handle.raw(), &admin(), &extension, GrantProfile::ReadOnly)
            .await
            .expect("grant read-only profile");
    assert!(conferred.is_empty(), "read-only confers no grants");

    let target = Id::from_raw("ext-invocation-x");
    let request = ControlRequest::new(
        ControlMethod::Invoke,
        target.clone(),
        serde_json::json!({ "action": "recompute" }),
    );

    let err = rubix_ext::invoke(handle.raw(), &extension, &request)
        .await
        .expect_err("an out-of-grant invoke must be denied");
    assert!(matches!(err, ExtError::Denied(_)));

    // No record and no audit row were written.
    let landed = rubix_core::read_record(handle.raw(), &target)
        .await
        .expect("read record");
    assert!(landed.is_none(), "no record on a denied invoke");
    assert_eq!(count_audit_rows(&handle, "readonly-ext").await, 0);
}
