//! POST /api/v1/widgets — pin a dashboard tile.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{Widget, WidgetKind};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWidget {
    pub site_id: Uuid,
    pub kind: WidgetKind,
    pub title: String,
    /// Point keyexpr (`point_*` kinds) or board slug (`board_output`).
    pub target: String,
}

#[utoipa::path(post, path = "/api/v1/widgets", request_body = CreateWidget, tag = "widgets",
    responses((status = 201, body = Widget), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn create_widget(
    State(state): State<AppState>,
    Json(req): Json<CreateWidget>,
) -> Result<(StatusCode, Json<Widget>), ApiError> {
    if req.title.trim().is_empty() || req.target.trim().is_empty() {
        return Err(ApiError::BadRequest("title and target must not be empty".into()));
    }
    let widget = Widget {
        id: Uuid::new_v4(),
        site_id: req.site_id,
        kind: req.kind,
        title: req.title,
        target: req.target,
        created_at: Utc::now(),
    };
    let stored = widget.clone();
    let store = state.store.clone();
    blocking(move || {
        store.create_widget(&stored)?;
        Ok(())
    })
    .await?;
    Ok((StatusCode::CREATED, Json(widget)))
}
