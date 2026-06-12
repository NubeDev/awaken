//! POST /api/v1/boards — store a board version.
//!
//! Persists the graph plus its trigger as a new `(slug, version)`. The trigger
//! is validated before insert; the version number is assigned server-side
//! (max existing + 1) so republishing is a plain re-POST of the same slug.
//! Newly stored scheduled boards take effect on the next scheduler launch —
//! the running scheduler is not hot-reconfigured (a restart picks them up).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::validate_slug;
use uuid::Uuid;

use super::dto::{BoardView, CreateBoard};
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::scheduler::BoardRecord;
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/boards", request_body = CreateBoard, tag = "boards",
    responses((status = 201, body = BoardView), (status = 400, body = ErrorBody)))]
pub(crate) async fn create_board(
    State(state): State<AppState>,
    Json(req): Json<CreateBoard>,
) -> Result<(StatusCode, Json<BoardView>), ApiError> {
    validate_slug(&req.slug)?;
    req.trigger.validate().map_err(ApiError::BadRequest)?;
    let store = state.store.clone();
    let record = blocking(move || {
        let version = store.next_board_version(&req.slug)?;
        let record = BoardRecord {
            id: Uuid::new_v4(),
            slug: req.slug,
            version,
            display_name: req.display_name,
            enabled: req.enabled,
            trigger: req.trigger,
            graph: req.board,
            created_at: Utc::now(),
        };
        store.create_board(&record)?;
        Ok(record)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(record.into())))
}
