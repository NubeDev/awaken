//! POST /api/v1/boards — store a board version.
//!
//! Persists the graph plus its trigger as a new `(slug, version)`. The trigger
//! is validated before insert; the version number is assigned server-side
//! (max existing + 1) so republishing is a plain re-POST of the same slug.
//! A scheduled board is registered with the running scheduler on create, so a
//! new or republished board starts (or restarts with its new cadence) without a
//! server restart.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::validate_slug;
use uuid::Uuid;

use super::dto::{BoardView, CreateBoard};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_board_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::scheduler::BoardRecord;
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/boards", request_body = CreateBoard, tag = "boards",
    security(("bearer" = [])),
    responses((status = 201, body = BoardView), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn create_board(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<CreateBoard>,
) -> Result<(StatusCode, Json<BoardView>), ApiError> {
    validate_slug(&req.org)?;
    validate_slug(&req.slug)?;
    req.trigger.validate().map_err(ApiError::BadRequest)?;
    authorize_board_write(&principal, &state.store, &req.org, req.site_id, "*")?;
    let store = state.store.clone();
    let record = blocking(move || {
        let version = store.next_board_version(&req.org, req.site_id, &req.slug)?;
        let record = BoardRecord {
            id: Uuid::new_v4(),
            org: req.org,
            site_id: req.site_id,
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
    // Start (or restart with the new graph/cadence) this board's loop now. A
    // manual or disabled board unregisters instead — no loop runs for it.
    if let Some(scheduler) = &state.scheduler {
        scheduler.register(&record);
    }
    Ok((StatusCode::CREATED, Json(record.into())))
}
