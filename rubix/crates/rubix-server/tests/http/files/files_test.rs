//! Integration: upload a file's bytes and download them, gated on `file-upload`.
//!
//! Proves build-order step 6 (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "File
//! fields"): `POST /files` stores bytes behind the blob store and returns a
//! reference (the shape a record's `file` field holds), and `GET /files/:id`
//! streams the bytes back — both scoped to the caller's namespace, with upload
//! gated on the fail-closed `file-upload` capability. The blob never crosses the
//! command gate; only the returned reference would, when stored in a record.

#[path = "../../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::Capability;
use tower::ServiceExt;

use fixture::app::{SECRET, SUBJECT, TestApp, boot};

/// The multipart boundary used by the hand-built upload bodies.
const BOUNDARY: &str = "rubixtestboundary";

/// Build a `multipart/form-data` body carrying one file part.
fn multipart_body(filename: &str, content_type: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// An authenticated multipart upload request as the fixture principal.
fn upload_request(filename: &str, content_type: &str, bytes: &[u8]) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/files")
        .header("x-rubix-subject", SUBJECT)
        .header("x-rubix-secret", SECRET)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(multipart_body(filename, content_type, bytes)))
        .expect("build upload request")
}

#[tokio::test]
async fn upload_then_download_round_trips_the_bytes() {
    let TestApp { app, .. } = boot("files_roundtrip", &[Capability::FileUpload]).await;

    // Upload returns the stored reference.
    let response = app
        .clone()
        .oneshot(upload_request(
            "plan.pdf",
            "application/pdf",
            b"%PDF-1.4 fake",
        ))
        .await
        .expect("upload responds");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let reference: serde_json::Value = serde_json::from_slice(&body).expect("reference json");
    assert_eq!(reference["filename"], "plan.pdf");
    assert_eq!(reference["contentType"], "application/pdf");
    assert_eq!(reference["size"], 13);
    let id = reference["id"].as_str().expect("id string").to_owned();

    // Download streams the exact bytes back with the stored content type.
    let download = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/files/{id}"))
                .header("x-rubix-subject", SUBJECT)
                .header("x-rubix-secret", SECRET)
                .body(Body::empty())
                .expect("build download"),
        )
        .await
        .expect("download responds");
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        download
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("application/pdf")
    );
    let bytes = download
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(&bytes[..], b"%PDF-1.4 fake");
}

#[tokio::test]
async fn upload_without_the_capability_is_forbidden() {
    // No FileUpload grant.
    let TestApp { app, .. } = boot("files_forbidden", &[]).await;

    let response = app
        .clone()
        .oneshot(upload_request("x.txt", "text/plain", b"nope"))
        .await
        .expect("upload responds");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn downloading_an_unknown_id_is_not_found() {
    let TestApp { app, .. } = boot("files_missing", &[Capability::FileUpload]).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/files/does-not-exist")
                .header("x-rubix-subject", SUBJECT)
                .header("x-rubix-secret", SECRET)
                .body(Body::empty())
                .expect("build download"),
        )
        .await
        .expect("download responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
