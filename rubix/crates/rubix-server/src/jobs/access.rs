//! The auth seam shared by every job endpoint: ticket resolution, the bulk-submit
//! capability gate, job registration, and ticket minting (`BULK-AND-JOBS.md`).
//!
//! These compose the gate's cryptographic ticket check with the server-owned
//! registry checks. The split is deliberate: [`resolve_job_ticket`] (in the gate)
//! proves the ticket is valid, unexpired, and bound to the addressed job; the
//! registry-existence and namespace-match checks live here because the in-memory
//! registry is the server's. Together they are the full validation the design
//! requires — and the registry-existence check is what makes a restart safe (an
//! orphan ticket whose job is gone resolves to "job unknown", a fail-closed deny).

use std::sync::Arc;
use std::time::Duration;

use rubix_gate::{
    Capability, DEFAULT_JOB_TICKET_TTL_SECONDS, check_capability, issue_job_ticket,
    resolve_job_ticket,
};

use crate::auth::Authenticated;
use crate::error::ApiError;
use crate::jobs::{Job, JobError, JobHandle, JobRegistry, ResultTransport};
use crate::state::AppState;

/// Resolve a presented ticket to the job it may observe, enforcing every gate.
///
/// Fails closed at each step: a ticket that is unknown/expired/wrong-job is a
/// rejection (`401`); a job absent from the registry (e.g. after a restart) is
/// "job unknown" (`404`, the client resubmits); a ticket whose namespace does not
/// match the job's tenant is a rejection. Bearer semantics: the presenter is *not*
/// re-checked against the original subject — any holder of a valid ticket may
/// observe that one job.
pub async fn resolve_observer(
    state: &AppState,
    job_id: &str,
    raw_ticket: &str,
) -> Result<Arc<Job>, ApiError> {
    let resolved = resolve_job_ticket(state.store.raw(), raw_ticket, job_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthenticated("invalid or expired job ticket".to_owned()))?;
    // Restart safety: the ticket row may outlive its job (the registry is in-memory
    // and empty after a restart). A missing job is "unknown", not a server error.
    let job = state.jobs.get(job_id).await.ok_or(ApiError::NotFound)?;
    if job.namespace != resolved.namespace {
        return Err(ApiError::Unauthenticated(
            "job ticket namespace does not match the job".to_owned(),
        ));
    }
    Ok(job)
}

/// Require the principal to hold `bulk-submit` before opening a job, fail closed.
///
/// Gates only the act of opening a bulk job — never the underlying mutations/reads,
/// which each item is still checked for individually through `apply()`.
pub async fn require_bulk_submit(state: &AppState, auth: &Authenticated) -> Result<(), ApiError> {
    if check_capability(state.store.raw(), &auth.principal, Capability::BulkSubmit)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "opening a bulk job requires the bulk-submit capability".to_owned(),
        ))
    }
}

/// Register a running job for the authenticated principal, mapping an over-cap
/// refusal to `429` (the request is refused, not queued).
pub async fn register_job(
    registry: &JobRegistry,
    auth: &Authenticated,
    transport: ResultTransport,
    total: u64,
) -> Result<JobHandle, ApiError> {
    registry
        .try_register(
            &auth.principal.namespace,
            &auth.principal.subject.to_string(),
            transport,
            total,
        )
        .await
        .map_err(|JobError::OverCapacity| {
            ApiError::TooManyRequests(
                "too many running jobs for this principal/namespace".to_owned(),
            )
        })
}

/// A freshly minted job ticket: the opaque value and its expiry.
pub struct MintedTicket {
    /// The opaque bearer value (returned to the client exactly once).
    pub value: String,
    /// When the ticket expires (RFC 3339, UTC).
    pub expires: String,
}

/// Mint the short-TTL ticket for a registered job, stamped from the authenticated
/// principal (tenant + subject), never from the request body.
///
/// On failure the job is left finalisable: the caller is expected to `fail` the
/// handle so it does not linger un-evictable.
pub async fn mint_ticket(
    state: &AppState,
    auth: &Authenticated,
    job_id: &str,
) -> Result<MintedTicket, ApiError> {
    let issued = issue_job_ticket(
        state.store.raw(),
        job_id,
        &auth.principal.subject.to_string(),
        &auth.principal.namespace,
        DEFAULT_JOB_TICKET_TTL_SECONDS,
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(MintedTicket {
        value: issued.value,
        expires: issued.expires,
    })
}

/// The cadence the synthetic job emits its progress steps at — small, so the spine
/// proof runs fast while still exercising the live stream + reconnect replay.
pub const SYNTHETIC_STEP_DELAY: Duration = Duration::from_millis(5);
