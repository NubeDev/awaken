//! GET /api/v1/points/{id}/his — time-range history query.

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use rubix_core::HisSample;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct HisQuery {
    /// Inclusive RFC 3339 lower bound.
    pub start: Option<DateTime<Utc>>,
    /// Exclusive RFC 3339 upper bound.
    pub end: Option<DateTime<Utc>>,
    /// Max samples returned (default 1000, cap 10000).
    pub limit: Option<usize>,
}

#[utoipa::path(get, path = "/api/v1/points/{id}/his", params(("id" = Uuid, Path), HisQuery),
    tag = "history",
    responses((status = 200, body = [HisSample]), (status = 404, body = ErrorBody)))]
pub(crate) async fn query_his(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<HisQuery>,
) -> Result<Json<Vec<HisSample>>, ApiError> {
    let limit = q.limit.unwrap_or(1000).min(10_000);
    Ok(Json(
        blocking(move || Ok(state.store.his_query(id, q.start, q.end, limit)?)).await?,
    ))
}
