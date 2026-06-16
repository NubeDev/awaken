//! Integration: the long-running job spine over HTTP (`BULK-AND-JOBS.md`).
//!
//! Drives the synthetic job through the public surface: submit → `202` handle,
//! poll the ticket to completion (with the buffered result), the concurrency cap
//! `429`, cancellation via `DELETE`, the grace-window eviction, and the
//! restart-safety job-absent `404`. The ticket authorizes the poll/cancel; a job
//! the registry no longer holds is "unknown", not a server error.

#[path = "../../fixture/mod.rs"]
mod fixture;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_gate::{Capability, issue_job_ticket};
use serde_json::{Value, json};
use tower::ServiceExt;

use fixture::app::{NS, SECRET, SUBJECT, TestJobApp, boot_jobs};
use rubix_server::jobs::JobLimits;

/// Send a request, returning the status and JSON body (or null on an empty body).
async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("route responds");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, json)
}

/// A credentialed JSON request (subject/secret headers).
fn authed(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-rubix-subject", SUBJECT)
        .header("x-rubix-secret", SECRET)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

/// A request carrying a job ticket as the bearer Authorization (the observer path).
fn ticketed(method: &str, uri: &str, ticket: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {ticket}"))
        .body(Body::empty())
        .expect("build request")
}

/// Generous limits + a non-zero deadline (the synthetic path ignores the deadline).
fn limits() -> JobLimits {
    JobLimits::default()
}

/// Poll a job until it leaves `running`, or panic after a bounded number of tries.
async fn poll_until_terminal(app: &axum::Router, job_id: &str, ticket: &str) -> Value {
    for _ in 0..200 {
        let (status, body) = send(
            app,
            ticketed("GET", &format!("/bulk/jobs/{job_id}"), ticket),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "poll ok: {body}");
        if body["status"] != json!("running") {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("job did not finish in time");
}

#[tokio::test]
async fn synthetic_job_submits_polls_and_completes_with_buffered_result() {
    let TestJobApp { app, .. } = boot_jobs(
        "jobs_spine",
        &[Capability::BulkSubmit],
        limits(),
        Duration::from_secs(10),
    )
    .await;

    let (status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 3 }))).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = accepted["job_id"].as_str().expect("job_id").to_owned();
    let ticket = accepted["ticket"].as_str().expect("ticket").to_owned();
    assert!(!accepted["expires"].as_str().expect("expires").is_empty());

    let terminal = poll_until_terminal(&app, &job_id, &ticket).await;
    assert_eq!(terminal["status"], json!("completed"));
    assert_eq!(terminal["result_transport"], json!("poll"));
    // The buffered per-item statuses are returned by the poll (poll-transport job).
    let result = terminal["result"].as_array().expect("buffered result");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0]["status"], json!("ok"));
}

#[tokio::test]
async fn submitting_without_bulk_submit_is_forbidden() {
    // No capabilities granted — opening a job is denied before registration.
    let TestJobApp { app, .. } =
        boot_jobs("jobs_no_cap", &[], limits(), Duration::from_secs(10)).await;
    let (status, _) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1 }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn over_the_running_cap_returns_429() {
    // A per-principal cap of one running job: the second concurrent submit is 429.
    let limits = JobLimits {
        max_running_per_principal: 1,
        ..JobLimits::default()
    };
    // Long steps so the first job stays running while the second submits.
    let TestJobApp { app, .. } = boot_jobs(
        "jobs_cap",
        &[Capability::BulkSubmit],
        limits,
        Duration::from_secs(10),
    )
    .await;

    let (status, _first) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1000 }))).await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let (status, _second) =
        send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1000 }))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn delete_cancels_the_job_and_revokes_the_ticket() {
    let TestJobApp { app, .. } = boot_jobs(
        "jobs_cancel",
        &[Capability::BulkSubmit],
        limits(),
        Duration::from_secs(10),
    )
    .await;

    let (_status, accepted) =
        send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1000 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();
    let ticket = accepted["ticket"].as_str().unwrap().to_owned();

    let (status, _) = send(
        &app,
        ticketed("DELETE", &format!("/bulk/jobs/{job_id}"), &ticket),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The ticket is revoked on DELETE, so a subsequent poll is rejected.
    let (status, _) = send(
        &app,
        ticketed("GET", &format!("/bulk/jobs/{job_id}"), &ticket),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_bad_ticket_is_rejected() {
    let TestJobApp { app, .. } = boot_jobs(
        "jobs_bad_ticket",
        &[Capability::BulkSubmit],
        limits(),
        Duration::from_secs(10),
    )
    .await;
    let (_status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();

    let (status, _) = send(
        &app,
        ticketed(
            "GET",
            &format!("/bulk/jobs/{job_id}"),
            "not-the-real-ticket",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_valid_ticket_for_an_absent_job_is_unknown() {
    // Restart safety: a ticket can outlive its job (the registry is in-memory and
    // empty after a restart). A gate-minted ticket whose job was never registered
    // resolves cryptographically but the job is absent → "job unknown" (404).
    let TestJobApp { app, state } = boot_jobs(
        "jobs_absent",
        &[Capability::BulkSubmit],
        limits(),
        Duration::from_secs(10),
    )
    .await;

    let issued = issue_job_ticket(state.store.raw(), "ghost-job", SUBJECT, NS, 3600)
        .await
        .expect("mint ticket for an unregistered job");

    let (status, _) = send(&app, ticketed("GET", "/bulk/jobs/ghost-job", &issued.value)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_terminal_job_is_pollable_through_the_grace_window_then_evicted() {
    // Zero grace: a sweep evicts a terminal job immediately, so we can assert the
    // before/after without sleeping.
    let limits = JobLimits {
        grace: Duration::ZERO,
        ..JobLimits::default()
    };
    let TestJobApp { app, state } = boot_jobs(
        "jobs_grace",
        &[Capability::BulkSubmit],
        limits,
        Duration::from_secs(10),
    )
    .await;

    let (_status, accepted) = send(&app, authed("POST", "/bulk/jobs", json!({ "steps": 1 }))).await;
    let job_id = accepted["job_id"].as_str().unwrap().to_owned();
    let ticket = accepted["ticket"].as_str().unwrap().to_owned();

    // Pollable while it lives (and through the grace window before a sweep runs).
    let terminal = poll_until_terminal(&app, &job_id, &ticket).await;
    assert_eq!(terminal["status"], json!("completed"));

    // The sweeper evicts the terminal job; the ticket now resolves to a gone job.
    let evicted = state.jobs.sweep(state.store.raw()).await;
    assert_eq!(evicted, 1);

    let (status, _) = send(
        &app,
        ticketed("GET", &format!("/bulk/jobs/{job_id}"), &ticket),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
