//! Opaque, server-side **job tickets** — the long-running-job observation surface
//! (`rubix/docs/design/BULK-AND-JOBS.md`, "Job ticket").
//!
//! A bulk op that promotes to a background job returns a short-TTL ticket the
//! client presents to observe that one job — over the WS job channel
//! (`Sec-WebSocket-Protocol`) or the status poll (`Authorization`). This store is
//! a **specialisation of [`auth_token`](crate::auth_token)**: same hashed-at-rest,
//! expiring, namespace-scoped, revocable posture, but a far narrower scope. The
//! choices are deliberate (`BULK-AND-JOBS.md`, "Job ticket — copy `session_token`"):
//!
//! - **A parallel table, not reuse of `session_token`.** A session token resolves
//!   to *full principal credentials*; a job ticket resolves only to "may observe
//!   this one job" — narrower scope, separate lifetime (minutes, not 24h). Reusing
//!   the session table would over-grant.
//! - **Opaque + hashed at rest + expiring**, exactly as `auth_token`: the row holds
//!   only `crypto::sha256(value)`, so a store read never yields a usable ticket, and
//!   resolution rejects an expired row.
//! - **Bearer semantics.** [`resolve_job_ticket`] does *not* re-check the presenter
//!   is the original `subject` — any holder of a valid ticket may observe that one
//!   job (so a client can hand the ticket to a worker/tab that opens the socket).
//!   This is bounded by the short TTL and single-job scope; the stored `subject` is
//!   for audit/attribution, not an access gate.
//! - **Revoked on expiry or eviction, *not* on job completion** — the ticket must
//!   outlive completion so the client can read terminal status during the grace
//!   window. Revocation is therefore keyed by `job_id` ([`revoke_job_ticket`]),
//!   because the server holds the job id (the raw value was returned to the client
//!   once and never stored), unlike `auth_token`'s logout which presents the raw.
//!
//! The registry-existence half of the validation ("the job still exists") lives in
//! the server, which owns the in-memory registry — this module proves only that the
//! ticket is cryptographically valid, unexpired, and bound to the addressed job.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, SurrealValue};

use rubix_core::Id;

use crate::error::{GateError, Result};

/// The table job tickets are stored in (defined by
/// [`define_gate_schema`](crate::define_gate_schema)).
const JOB_TICKET_TABLE: &str = "job_ticket";

/// Default job-ticket lifetime: 15 minutes. Sized to cover the expected job
/// duration plus the eviction grace window, so a client can still read terminal
/// status after the job finishes; an explicit `DELETE /bulk/jobs/{id}` revokes
/// early. Short by design — the blast radius of a leaked ticket is one job.
pub const DEFAULT_JOB_TICKET_TTL_SECONDS: i64 = 15 * 60;

/// A freshly minted job ticket returned to the client exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedJobTicket {
    /// The opaque bearer value the client carries (never stored in the clear).
    pub value: String,
    /// When the ticket expires (RFC 3339, UTC).
    pub expires: String,
}

/// The claims a job ticket resolves back to.
///
/// Deliberately narrow: the job the ticket may observe, its tenant, and the
/// subject that minted it (for audit, not as an access gate — see bearer
/// semantics in the module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedJobTicket {
    /// The job id the ticket grants observation of.
    pub job_id: String,
    /// The namespace the job (and ticket) belongs to.
    pub namespace: String,
    /// The subject that minted the ticket (audit/attribution only).
    pub subject: String,
}

/// The fields persisted for a ticket (decoded on resolve).
#[derive(Debug, SurrealValue)]
struct TicketRow {
    job_id: String,
    namespace: String,
    subject: String,
}

/// The `expires` projection returned by the issue statement.
#[derive(Debug, SurrealValue)]
struct ExpiresRow {
    expires: Datetime,
}

/// Mint an opaque ticket granting observation of one job.
///
/// The raw value is generated here and never stored — only its SHA-256 — so this
/// return is the one and only time it is available in the clear. Tenancy
/// (`namespace`) and `subject` are stamped from the authenticated principal by the
/// caller, never from a request body (mirrors readings/records).
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the write fails or its result cannot be
/// decoded.
pub async fn issue_job_ticket(
    db: &Surreal<Db>,
    job_id: &str,
    subject: &str,
    namespace: &str,
    ttl_seconds: i64,
) -> Result<IssuedJobTicket> {
    // 256 bits of entropy from two v4 UUIDs — opaque and unguessable.
    let raw = format!("{}{}", Id::new().as_str(), Id::new().as_str());
    let ttl = format!("{ttl_seconds}s");

    let mut response = db
        .query(format!(
            "CREATE {JOB_TICKET_TABLE} SET \
               token_hash = crypto::sha256($raw), \
               job_id = $job_id, \
               subject = $subject, \
               namespace = $namespace, \
               expires = time::now() + type::duration($ttl) \
             RETURN expires"
        ))
        .bind(("raw", raw.clone()))
        .bind(("job_id", job_id.to_owned()))
        .bind(("subject", subject.to_owned()))
        .bind(("namespace", namespace.to_owned()))
        .bind(("ttl", ttl))
        .await
        .map_err(GateError::TokenStore)?;
    let row: Option<ExpiresRow> = response.take(0).map_err(GateError::TokenStore)?;
    let expires = row
        .map(|r| r.expires.to_string())
        .ok_or_else(|| GateError::Authenticate("job ticket issue returned no row".to_owned()))?;

    Ok(IssuedJobTicket {
        value: raw,
        expires,
    })
}

/// Resolve an opaque job ticket presented for `job_id`.
///
/// Matches on the ticket's SHA-256 and requires the row to be **unexpired** and
/// its stored `job_id` to equal the addressed `job_id`, so an unknown, revoked,
/// expired, or wrong-job ticket resolves to `None` (the caller rejects it). The
/// raw ticket never touches the store — only its hash, computed in the engine.
///
/// The final "the job still exists in the registry" check (restart safety) is the
/// server's, since it owns the in-memory registry; a `Some` here means only that
/// the ticket is cryptographically valid and bound to the addressed job.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the lookup query fails.
pub async fn resolve_job_ticket(
    db: &Surreal<Db>,
    raw: &str,
    job_id: &str,
) -> Result<Option<ResolvedJobTicket>> {
    let mut response = db
        .query(format!(
            "SELECT job_id, namespace, subject FROM {JOB_TICKET_TABLE} \
             WHERE token_hash = crypto::sha256($raw) \
               AND job_id = $job_id \
               AND expires > time::now() \
             LIMIT 1"
        ))
        .bind(("raw", raw.to_owned()))
        .bind(("job_id", job_id.to_owned()))
        .await
        .map_err(GateError::TokenStore)?;
    let row: Option<TicketRow> = response.take(0).map_err(GateError::TokenStore)?;
    Ok(row.map(|r| ResolvedJobTicket {
        job_id: r.job_id,
        namespace: r.namespace,
        subject: r.subject,
    }))
}

/// Revoke every ticket for `job_id`, returning whether any was deleted.
///
/// Keyed by `job_id` (not the raw value) because the server holds the job id — the
/// raw ticket was returned to the client once and never stored. Called on explicit
/// `DELETE /bulk/jobs/{id}` and when the sweeper evicts a terminal job, so the
/// ticket and job lifetimes stay aligned. Idempotent: revoking a job with no live
/// ticket returns `false` rather than erroring.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the delete query fails.
pub async fn revoke_job_ticket(db: &Surreal<Db>, job_id: &str) -> Result<bool> {
    let mut response = db
        .query(format!(
            "DELETE {JOB_TICKET_TABLE} WHERE job_id = $job_id RETURN BEFORE"
        ))
        .bind(("job_id", job_id.to_owned()))
        .await
        .map_err(GateError::TokenStore)?;
    let deleted: Vec<serde_json::Value> = response.take(0).map_err(GateError::TokenStore)?;
    Ok(!deleted.is_empty())
}

/// Delete every expired job-ticket row, returning how many were swept.
///
/// Run periodically so orphan ticket rows (whose job is gone, e.g. after a server
/// restart that emptied the registry) do not accumulate. An orphan whose `expires`
/// is still in the future is harmless — it resolves to `None` at the server's
/// registry-existence check — and is reaped here once it expires.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the delete query fails.
pub async fn sweep_expired_job_tickets(db: &Surreal<Db>) -> Result<u64> {
    let mut response = db
        .query(format!(
            "DELETE {JOB_TICKET_TABLE} WHERE expires <= time::now() RETURN BEFORE"
        ))
        .await
        .map_err(GateError::TokenStore)?;
    let deleted: Vec<serde_json::Value> = response.take(0).map_err(GateError::TokenStore)?;
    Ok(deleted.len() as u64)
}
