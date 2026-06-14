//! Units & datetime preferences (WS-11) route tests. Auth is off in the harness
//! (edge posture), so `/me/preferences` resolves under the `default` org with
//! the synthetic `dev` user — exactly the dev-UI path.

use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

/// With no stored prefs, `/me/preferences` returns the system defaults
/// (en-US, UTC, metric → celsius, 24h, system theme).
#[tokio::test]
async fn me_preferences_default_to_system() {
    let app = TestApp::new();
    let (status, body) = app.request("GET", "/api/v1/me/preferences", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["locale"], "en-US");
    assert_eq!(body["unit_system"], "metric");
    assert_eq!(body["temperature_unit"], "celsius");
    assert_eq!(body["time_format"], "24h");
    assert_eq!(body["theme"], "system");
}

/// A PATCH persists and the re-resolved view reflects it; `unit_system:
/// imperial` flips the unsplit-by-system quantities (temperature → fahrenheit)
/// while an explicit per-unit override wins.
#[tokio::test]
async fn me_patch_persists_and_resolves() {
    let app = TestApp::new();
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me/preferences",
            Some(json!({
                "unit_system": "imperial",
                "speed_unit": "knot",
                "theme": "dark"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // imperial flips temperature; explicit speed override wins over the
    // imperial default (mile_per_hour).
    assert_eq!(body["unit_system"], "imperial");
    assert_eq!(body["temperature_unit"], "fahrenheit");
    assert_eq!(body["speed_unit"], "knot");
    assert_eq!(body["theme"], "dark");

    // Re-read confirms persistence.
    let (status, body) = app.request("GET", "/api/v1/me/preferences", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["temperature_unit"], "fahrenheit");
    assert_eq!(body["speed_unit"], "knot");
}

/// An explicit JSON `null` reverts a field to inherit (back to the system
/// default), distinct from omitting the key.
#[tokio::test]
async fn me_patch_null_reverts_to_inherit() {
    let app = TestApp::new();
    app.request(
        "PATCH",
        "/api/v1/me/preferences",
        Some(json!({"temperature_unit": "fahrenheit"})),
    )
    .await;
    // Now revert it.
    let (status, body) = app
        .request(
            "PATCH",
            "/api/v1/me/preferences",
            Some(json!({"temperature_unit": null})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["temperature_unit"], "celsius"); // back to system default
}

/// An unknown field is a 400, not a silent no-op.
#[tokio::test]
async fn me_patch_rejects_unknown_field() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/me/preferences",
            Some(json!({"nonsense": "x"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// `/units` exposes the closed registry: every quantity with its canonical and
/// allowed wire units.
#[tokio::test]
async fn units_document_lists_the_registry() {
    let app = TestApp::new();
    let (status, body) = app.request("GET", "/api/v1/units", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let quantities = body["quantities"].as_array().expect("quantities array");
    let temp = quantities
        .iter()
        .find(|q| q["quantity"] == "temperature")
        .expect("temperature entry");
    assert_eq!(temp["canonical"], "celsius");
    let allowed: Vec<&str> = temp["allowed"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(allowed.contains(&"fahrenheit"));
}
