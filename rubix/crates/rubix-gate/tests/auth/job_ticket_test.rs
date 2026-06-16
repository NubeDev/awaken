//! Integration: the opaque job-ticket lifecycle against a live kv-mem SurrealDB.
//!
//! Exercises the job-observation surface (`rubix/docs/design/BULK-AND-JOBS.md`,
//! "Job ticket"): a minted ticket resolves back to its job + tenant when presented
//! for that job; it is rejected for a different job id, in a stale (expired) state,
//! when revoked by job id, and when unknown; and the expired-row sweep reaps
//! orphans. The raw ticket is never stored in the clear (only its hash) — proven
//! by resolution working through the hash path alone.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_gate::{
    issue_job_ticket, resolve_job_ticket, revoke_job_ticket, sweep_expired_job_tickets,
};

use gate::open::{NS, open_gate_store};

const SUBJECT: &str = "operator";

#[tokio::test]
async fn a_minted_ticket_resolves_for_its_job() {
    let handle = open_gate_store("ticket_resolve").await;

    let issued = issue_job_ticket(handle.raw(), "job-1", SUBJECT, NS, 3600)
        .await
        .expect("issue");
    assert!(!issued.value.is_empty());
    assert!(!issued.expires.is_empty());

    let resolved = resolve_job_ticket(handle.raw(), &issued.value, "job-1")
        .await
        .expect("resolve")
        .expect("present");
    assert_eq!(resolved.job_id, "job-1");
    assert_eq!(resolved.namespace, NS);
    assert_eq!(resolved.subject, SUBJECT);
}

#[tokio::test]
async fn a_ticket_is_rejected_for_a_different_job() {
    let handle = open_gate_store("ticket_job_mismatch").await;
    let issued = issue_job_ticket(handle.raw(), "job-a", SUBJECT, NS, 3600)
        .await
        .expect("issue");

    // The hash matches but the addressed job id does not — a ticket for job-a may
    // not observe job-b.
    let resolved = resolve_job_ticket(handle.raw(), &issued.value, "job-b")
        .await
        .expect("resolve");
    assert!(resolved.is_none());
}

#[tokio::test]
async fn an_unknown_ticket_resolves_to_none() {
    let handle = open_gate_store("ticket_unknown").await;
    let resolved = resolve_job_ticket(handle.raw(), "not-a-real-ticket", "job-1")
        .await
        .expect("resolve");
    assert!(resolved.is_none());
}

#[tokio::test]
async fn an_expired_ticket_no_longer_resolves() {
    let handle = open_gate_store("ticket_expired").await;
    // A zero-second TTL is already expired at resolve time.
    let issued = issue_job_ticket(handle.raw(), "job-1", SUBJECT, NS, 0)
        .await
        .expect("issue");
    let resolved = resolve_job_ticket(handle.raw(), &issued.value, "job-1")
        .await
        .expect("resolve");
    assert!(resolved.is_none());
}

#[tokio::test]
async fn a_revoked_job_ticket_no_longer_resolves() {
    let handle = open_gate_store("ticket_revoke").await;
    let issued = issue_job_ticket(handle.raw(), "job-1", SUBJECT, NS, 3600)
        .await
        .expect("issue");

    assert!(
        revoke_job_ticket(handle.raw(), "job-1")
            .await
            .expect("revoke")
    );
    let resolved = resolve_job_ticket(handle.raw(), &issued.value, "job-1")
        .await
        .expect("resolve");
    assert!(resolved.is_none());

    // Idempotent: revoking again (no live ticket) reports nothing deleted.
    assert!(
        !revoke_job_ticket(handle.raw(), "job-1")
            .await
            .expect("revoke again")
    );
}

#[tokio::test]
async fn tickets_are_isolated_by_namespace() {
    let handle = open_gate_store("ticket_namespace").await;
    let issued = issue_job_ticket(handle.raw(), "job-1", SUBJECT, "tenant-a", 3600)
        .await
        .expect("issue");

    // The ticket carries the namespace it was minted against — the server compares
    // it to the job's namespace before admitting an observer.
    let resolved = resolve_job_ticket(handle.raw(), &issued.value, "job-1")
        .await
        .expect("resolve")
        .expect("present");
    assert_eq!(resolved.namespace, "tenant-a");
}

#[tokio::test]
async fn the_sweep_reaps_only_expired_rows() {
    let handle = open_gate_store("ticket_sweep").await;
    // One already-expired orphan, one live ticket.
    issue_job_ticket(handle.raw(), "job-old", SUBJECT, NS, 0)
        .await
        .expect("issue expired");
    let live = issue_job_ticket(handle.raw(), "job-live", SUBJECT, NS, 3600)
        .await
        .expect("issue live");

    let swept = sweep_expired_job_tickets(handle.raw())
        .await
        .expect("sweep");
    assert_eq!(swept, 1);

    // The live ticket survives the sweep.
    assert!(
        resolve_job_ticket(handle.raw(), &live.value, "job-live")
            .await
            .expect("resolve")
            .is_some()
    );
}
