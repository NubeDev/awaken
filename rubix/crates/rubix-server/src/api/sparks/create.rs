//! POST /api/v1/sparks — record a rule finding.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, Spark, SparkSeverity};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSpark {
    pub site_id: Uuid,
    /// Rule identity, the `{rule}` segment of `{org}/{site}/spark/{rule}/**`.
    pub rule: String,
    pub severity: SparkSeverity,
    pub message: String,
    #[serde(default)]
    pub point_ids: Vec<Uuid>,
}

#[utoipa::path(post, path = "/api/v1/sparks", request_body = CreateSpark, tag = "sparks",
    responses((status = 201, body = Spark), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn create_spark(
    State(state): State<AppState>,
    Json(req): Json<CreateSpark>,
) -> Result<(StatusCode, Json<Spark>), ApiError> {
    validate_slug(&req.rule)?;
    let spark = Spark {
        id: Uuid::new_v4(),
        site_id: req.site_id,
        rule: req.rule,
        severity: req.severity,
        message: req.message,
        point_ids: req.point_ids,
        ts: Utc::now(),
        acknowledged: false,
    };
    let stored = spark.clone();
    blocking(move || Ok(state.store.create_spark(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(spark)))
}
