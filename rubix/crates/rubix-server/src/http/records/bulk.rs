//! `POST /records/bulk` — bulk record create/update/delete (`BULK-AND-JOBS.md`,
//! "Bulk record CRUD").
//!
//! An envelope of keyed items, each gated and audited **individually** through the
//! gate's `apply()` — bulk is a server-side fan-out over the existing single-item
//! command path, never a permission shortcut. The `bulk-submit` capability gates
//! the *resource* (opening the bulk op); each item is still checked against its own
//! per-item write capability + row-level perms inside `apply()`, so a principal with
//! `bulk-submit` but no record-write cap gets every item forbidden.
//!
//! Two tiers behind one server-decided deadline (the client handles both the same):
//!
//! - **Tier 1 (sync, HTTP 200):** loop `apply()` per item with per-item isolation;
//!   the envelope is `200` unless it is itself malformed. A single bad item comes
//!   back `failed` with its error while the rest commit.
//! - **Tier 2 (promoted, HTTP 202):** if the soft deadline is exceeded mid-flight
//!   (or the caller forced `mode: "async"`), the remaining items run in a background
//!   job that emits each `{ key, status }` as a WS frame. The `202` body carries the
//!   statuses of every item that committed **before** promotion, so the union of the
//!   body and the frames is the full result — keyed by `key`, no gap, no double-report.

use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rubix_core::{Id, Principal};
use rubix_gate::{Change, Command, GateError, apply};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::auth::Authenticated;
use crate::dto::bulk::{
    BulkItemStatus, BulkMode, BulkOp, BulkPromotedResponse, BulkRecordItem, BulkRecordsRequest,
    BulkRecordsResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::http::records::capability::RECORD_WRITE;
use crate::jobs::{ResultTransport, drive, mint_ticket, register_job, require_bulk_submit};
use crate::state::AppState;

/// The largest number of items one bulk envelope may carry — bounded so a single
/// request cannot pin unbounded work (OQ1; conservative starting point).
const MAX_ITEMS: usize = 500;

/// Apply a bulk record envelope, synchronously or as a promoted background job.
///
/// `403` if the principal lacks `bulk-submit`; `400` if the envelope is empty or
/// over the item cap. Otherwise `200` with every item's status (Tier 1), or `202`
/// with a job handle + the already-committed statuses (Tier 2). Note a "sync"
/// endpoint can hand back a job handle — the client accepts either shape.
pub async fn bulk_records_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<BulkRecordsRequest>,
) -> ApiResult<Response> {
    require_bulk_submit(&state, &auth).await?;

    if body.items.is_empty() {
        return Err(ApiError::BadRequest(
            "a bulk request must carry at least one item".to_owned(),
        ));
    }
    if body.items.len() > MAX_ITEMS {
        return Err(ApiError::BadRequest(format!(
            "a bulk request may carry at most {MAX_ITEMS} items, got {}",
            body.items.len()
        )));
    }

    // Forced async (heavy-work hint): promote before committing anything, so the
    // whole envelope streams over WS and the 202 carries no committed items.
    if body.mode == BulkMode::Async {
        return promote(&state, &auth, Vec::new(), body.items).await;
    }

    // Tier 1: apply inline with per-item isolation, watching the soft deadline. The
    // deadline is checked *after* each commit so a promotion always carries at least
    // the item that tripped it — the union with the WS stream stays complete.
    let started = Instant::now();
    let mut committed = Vec::with_capacity(body.items.len());
    let mut remaining = body.items.into_iter();
    let mut promoted_any = false;

    for item in remaining.by_ref() {
        committed.push(apply_item(state.store.raw(), &auth.principal, item).await);
        if started.elapsed() >= state.bulk_deadline {
            promoted_any = true;
            break;
        }
    }

    let rest: Vec<BulkRecordItem> = remaining.collect();
    if promoted_any && !rest.is_empty() {
        // Some items committed inline; the rest run in a job. Invalidate now for the
        // inline writes; the job invalidates again when it finishes.
        invalidate(&state, &auth.principal);
        return promote(&state, &auth, committed, rest).await;
    }

    // Everything finished within the deadline — Tier 1.
    invalidate(&state, &auth.principal);
    Ok((
        StatusCode::OK,
        Json(BulkRecordsResponse { items: committed }),
    )
        .into_response())
}

/// Promote the remaining items to a background job, returning `202` with the
/// already-committed statuses and a ticket to observe the rest over WS.
async fn promote(
    state: &AppState,
    auth: &Authenticated,
    committed: Vec<BulkItemStatus>,
    rest: Vec<BulkRecordItem>,
) -> ApiResult<Response> {
    let total = rest.len() as u64;
    // CRUD per-item statuses are small + bounded, so they buffer for the poll.
    let handle = register_job(&state.jobs, auth, ResultTransport::Poll, total).await?;
    let job_id = handle.id().to_owned();

    let ticket = match mint_ticket(state, auth, &job_id).await {
        Ok(ticket) => ticket,
        Err(error) => {
            handle.fail("ticket issuance failed".to_owned());
            return Err(error);
        }
    };

    // The spawned job owns the remaining items; it captures its own store handle,
    // principal, and cache so it outlives the request.
    let store = state.store.clone();
    let principal = auth.principal.clone();
    let cache = state.context_cache.clone();
    let namespace = auth.principal.namespace.clone();
    drive(handle, move |handle| async move {
        for item in rest {
            if handle.is_cancelled() {
                return Err("cancelled".to_owned());
            }
            let status = apply_item(store.raw(), &principal, item).await;
            handle.item(status.key, &status.status, status.id, status.error);
        }
        // The job's writes are a data-change signal too — re-scan on next tick.
        cache.invalidate_namespace(&namespace);
        Ok(())
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(BulkPromotedResponse {
            job_id,
            ticket: ticket.value,
            expires: ticket.expires,
            committed,
        }),
    )
        .into_response())
}

/// Apply one bulk item through the gate, isolating its outcome to its own status.
///
/// Each item is a full single-item command: its own capability check, validation,
/// atomic capture, and audit row. A denial/validation/apply failure becomes this
/// item's `failed` status, never the envelope's.
async fn apply_item(
    db: &Surreal<Db>,
    principal: &Principal,
    item: BulkRecordItem,
) -> BulkItemStatus {
    let BulkRecordItem { key, op, body } = item;
    let content = body.unwrap_or_else(|| serde_json::json!({}));

    let (target, change, verb, id) = match op {
        BulkOp::Create => {
            // The key is a client correlation tag; the stored id is minted here.
            let id = Id::new();
            (
                id.clone(),
                Change::Create(content),
                "created",
                Some(id.to_string()),
            )
        }
        BulkOp::Update => {
            let id = Id::from_raw(key.clone());
            (id, Change::Update(content), "updated", Some(key.clone()))
        }
        BulkOp::Delete => {
            let id = Id::from_raw(key.clone());
            (id, Change::Delete, "deleted", Some(key.clone()))
        }
    };

    let command = Command::new(principal.clone(), RECORD_WRITE, target, change);
    match apply(db, &command, None).await {
        Ok(_) => BulkItemStatus::committed(key, verb, id),
        Err(error) => BulkItemStatus::failed(key, item_error(error)),
    }
}

/// A readable per-item error string from a gate failure (denial/validation reason,
/// or the error's display form for an unexpected store/apply failure).
fn item_error(error: GateError) -> String {
    match error {
        GateError::CommandDenied(reason) => format!("forbidden: {reason}"),
        GateError::Validation(reason) => format!("unprocessable: {reason}"),
        other => other.to_string(),
    }
}

/// Evict the writing principal's namespace from the scanned-context cache so a
/// board re-scans on its next tick (mirrors the single-record mutation routes).
fn invalidate(state: &AppState, principal: &Principal) {
    state
        .context_cache
        .invalidate_namespace(&principal.namespace);
}
