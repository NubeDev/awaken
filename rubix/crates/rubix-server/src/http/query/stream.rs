//! Streaming query — promote a wide read to a Tier-2 job (`BULK-AND-JOBS.md`,
//! "Streaming query (layer 3) — the timeseries driver").
//!
//! A wide reading scan cannot be one JSON blob, so the fix is to stop flattening at
//! the HTTP boundary: instead of `collect()` → one array, a streamed query job
//! pulls `DataFrame::execute_stream` and pushes each `RecordBatch` as a WS chunk
//! frame, closed by the terminal `Done`. The engine is already chunked; only the
//! boundary changes. The poll for these jobs is status-only (`result_transport:
//! "stream"`) — the rows are never fully buffered, so they are available only over
//! the WS stream.
//!
//! Opening the stream is cheap (authorize via `external-query` + plan), so the
//! `202` returns promptly; the heavy scan happens as the spawned job pumps. The
//! `external-query` capability gates the *read* (inside `span_stream`); `bulk-submit`
//! gates opening the *job*.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use rubix_datasource::span_stream;

use crate::auth::Authenticated;
use crate::dto::job::JobAcceptedDto;
use crate::dto::query::QueryRequest;
use crate::error::ApiResult;
use crate::http::query::render::{batches_to_rows, columns_of_schema};
use crate::http::query::run::{
    map_query_error, map_time_error, resolve_query, resolve_request_sql,
};
use crate::jobs::{ResultTransport, drive, mint_ticket, register_job, require_bulk_submit};
use crate::state::AppState;

/// Promote a query to a streamed Tier-2 job and return its `202` handle.
///
/// `403` if the principal lacks `bulk-submit` (opening the job) or `external-query`
/// (the read); `400` for a malformed statement/time scope; `429` over the job cap.
/// On success the rows arrive only as chunked WS frames + a terminal `Done`.
pub async fn promote_query_route(
    state: AppState,
    auth: Authenticated,
    body: QueryRequest,
) -> ApiResult<Response> {
    require_bulk_submit(&state, &auth).await?;

    // Resolve saved-query id + time macros + variables to the final statement,
    // exactly as the inline path, before opening the stream.
    let body = resolve_request_sql(&auth, body).await?;
    let resolved = resolve_query(body).map_err(map_time_error)?;

    // Build the lazy result stream while the request's borrows are live; it owns its
    // plan, so it outlives them and the spawned job pumps it.
    let stream = {
        let registry = state.datasources.read().await;
        span_stream(
            &registry,
            state.store.raw(),
            &auth.session,
            &state.context_cache,
            &resolved.sql,
        )
        .await
        .map_err(map_query_error)?
    };
    let columns = columns_of_schema(&stream.schema());

    // Total rows are unknown up front for a stream, so seed progress with 0.
    let handle = register_job(&state.jobs, &auth, ResultTransport::Stream, 0).await?;
    let job_id = handle.id().to_owned();
    let ticket = match mint_ticket(&state, &auth, &job_id).await {
        Ok(ticket) => ticket,
        Err(error) => {
            handle.fail("ticket issuance failed".to_owned());
            return Err(error);
        }
    };

    drive(handle, move |handle| async move {
        let mut stream = stream;
        let mut first = true;
        while let Some(batch) = stream.next().await {
            if handle.is_cancelled() {
                return Err("cancelled".to_owned());
            }
            let batch = batch.map_err(|e| e.to_string())?;
            // Columns ride the first chunk so the client gets types without sniffing.
            let chunk_columns = first.then(|| columns.clone());
            let rows = batches_to_rows(&[batch])?;
            handle.chunk(rows, chunk_columns);
            first = false;
        }
        // An empty result still reports its schema once, so the client renders the
        // right axes (mirrors the inline path's empty-result column list).
        if first {
            handle.chunk(Vec::new(), Some(columns));
        }
        Ok(())
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(JobAcceptedDto {
            job_id,
            ticket: ticket.value,
            expires: ticket.expires,
        }),
    )
        .into_response())
}
