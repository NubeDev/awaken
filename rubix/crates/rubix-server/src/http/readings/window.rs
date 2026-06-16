//! `GET /readings` — the windowed, series-scoped historian read.
//!
//! The read side of the data plane (`rubix/docs/design/READINGS-TIMESERIES.md`,
//! "Read path"). A chart asks for one `series` over one `[from, to]` window; this
//! issues the **filtered SurrealQL** range read (`WHERE series = $s AND at BETWEEN
//! $t0 AND $t1 ORDER BY at`) the `(namespace, series, at)` index serves — never a
//! fetch-the-whole-collection scan. It runs on the WS-03 scoped session, so the
//! `reading` row permission confines the result to the principal's namespace
//! natively (contract #1); the windowed read carries no capability of its own.
//! Rows come back raw and `at`-ordered — bucketing on `at` happens above this (the
//! query layer or the client), which is the trend-collapse fix.

use axum::Json;
use axum::extract::Query;
use rubix_gate::read_readings_on_session;
use serde::Deserialize;
use surrealdb::types::Datetime;

use crate::auth::Authenticated;
use crate::dto::reading::ReadingDto;
use crate::error::{ApiError, ApiResult};

/// The window a historian read is scoped to: one series, a `[from, to]` bound.
#[derive(Debug, Deserialize)]
pub struct ReadingWindowQuery {
    /// The series-defining record id to read.
    series: String,
    /// The inclusive window start (RFC 3339, UTC).
    from: String,
    /// The inclusive window end (RFC 3339, UTC).
    to: String,
}

/// Read one series' readings in the requested window, `at`-ordered.
///
/// A malformed `from`/`to` instant is `400`; the series-scoping and namespace
/// fence are enforced by SurrealDB on the scoped session, so a series in another
/// namespace simply yields no rows.
pub async fn read_readings_route(
    auth: Authenticated,
    Query(query): Query<ReadingWindowQuery>,
) -> ApiResult<Json<Vec<ReadingDto>>> {
    let from: Datetime = query
        .from
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("`from` is not RFC 3339: {}", query.from)))?;
    let to: Datetime = query
        .to
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("`to` is not RFC 3339: {}", query.to)))?;

    let readings = read_readings_on_session(&auth.session, &query.series, &from, &to)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(readings.into_iter().map(ReadingDto::from).collect()))
}
