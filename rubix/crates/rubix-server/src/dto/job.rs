//! Wire shapes for the long-running job surface (`BULK-AND-JOBS.md`).
//!
//! The job spine speaks one mental model across every bulk op: a submission either
//! returns a completed result *or* a job handle (`202 { job_id, ticket, expires }`),
//! and the client observes the job over the WS plane or the status poll. These
//! DTOs are that handle, the poll's status view, and the trivial synthetic-job
//! submission used to prove the spine.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::jobs::{JobStatus, ResultTransport};

/// The body of a synthetic-job submission (`POST /bulk/jobs`).
///
/// The spine's proof harness: spawns a trivial background job that emits `steps`
/// per-item progress frames then completes, exercising registration → ticket →
/// WS stream → poll → eviction end to end without a real workload behind it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SubmitJobRequest {
    /// How many synthetic progress steps the job emits before completing
    /// (defaults to a small count).
    #[serde(default)]
    pub steps: Option<u32>,
}

/// The `202 Accepted` body for a promoted/submitted job: the handle the client
/// observes the job through.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobAcceptedDto {
    /// The job's id (the WS path and poll path address it).
    pub job_id: String,
    /// The opaque, short-TTL ticket presented to observe the job (returned once).
    pub ticket: String,
    /// When the ticket expires (RFC 3339, UTC).
    pub expires: String,
}

/// The status poll response (`GET /bulk/jobs/{id}`).
///
/// Always carries status; carries the buffered terminal `result` only for jobs
/// whose result is small and fully buffered (CRUD per-item statuses). For streamed
/// query jobs `result_transport` is `stream` and `result` is absent — the rows are
/// available only over the WS stream.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobStatusDto {
    /// The job's id.
    pub job_id: String,
    /// The lifecycle state: `running`, `completed`, or `failed`.
    pub status: String,
    /// Units completed so far (present while running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<u64>,
    /// Total units, when known (present while running and non-zero).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Why the job failed (present only for a `failed` status).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// How the terminal result reaches the client: `poll` (inline below) or
    /// `stream` (consume the WS).
    pub result_transport: String,
    /// The buffered terminal result, for a `poll`-transport job (the per-item
    /// status list). Absent for a streamed job, or while still running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Value>>,
}

impl JobStatusDto {
    /// Project a registry job's status into its wire view.
    #[must_use]
    pub fn project(
        job_id: String,
        status: &JobStatus,
        transport: ResultTransport,
        result: Option<Vec<Value>>,
    ) -> Self {
        let (status_str, done, total, reason) = match status {
            JobStatus::Running { done, total } => {
                ("running", Some(*done), (*total > 0).then_some(*total), None)
            }
            JobStatus::Completed => ("completed", None, None, None),
            JobStatus::Failed { reason } => ("failed", None, None, Some(reason.clone())),
        };
        Self {
            job_id,
            status: status_str.to_owned(),
            done,
            total,
            reason,
            result_transport: transport.as_str().to_owned(),
            // Only a terminal poll-transport job exposes its buffered result.
            result: if status.is_running() { None } else { result },
        }
    }
}
