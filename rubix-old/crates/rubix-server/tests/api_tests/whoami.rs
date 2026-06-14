use axum::http::StatusCode;

use super::harness::TestApp;

/// With auth off (the test harness default), `whoami` synthesizes a global
/// operator so the dev UI is fully enabled, and flags `auth_enabled: false`.
#[tokio::test]
async fn whoami_synthesizes_dev_admin_when_auth_off() {
    let app = TestApp::new();
    let (status, body) = app.request("GET", "/api/v1/whoami", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["subject"], "dev");
    assert_eq!(body["role"], "operator");
    assert_eq!(body["can_write"], true);
    assert_eq!(body["auth_enabled"], false);
}
