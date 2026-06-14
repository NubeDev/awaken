//! POST /api/v1/points/{id}/his — batch sample ingest (idempotent on ts).

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::HisSample;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HisInsertResponse {
    pub inserted: usize,
}

#[utoipa::path(post, path = "/api/v1/points/{id}/his", params(("id" = Uuid, Path)),
    request_body = Vec<HisSample>, tag = "history",
    responses((status = 200, body = HisInsertResponse), (status = 404, body = ErrorBody)))]
pub(crate) async fn insert_his(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(samples): Json<Vec<HisSample>>,
) -> Result<Json<HisInsertResponse>, ApiError> {
    Ok(Json(
        blocking(move || {
            let inserted = state.store.his_insert(id, &samples)?;
            Ok(HisInsertResponse { inserted })
        })
        .await?,
    ))
}
