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
    /// Dashboard to pin onto. When omitted, the tile lands on `site_id`'s
    /// default dashboard (created on demand) — the legacy "pin to site" path.
    #[serde(default)]
    pub dashboard_id: Option<Uuid>,
    pub site_id: Uuid,
    pub kind: WidgetKind,
    pub title: String,
    /// Point keyexpr (`point_*` kinds), board slug (`board_output`), or
    /// datasource id (`datasource`).
    pub target: String,
    /// Native SQL for a `datasource` tile (required for that kind, rejected for
    /// every other). Operator-authored — the same trust tier as a spark node.
    #[serde(default)]
    pub query: Option<String>,
}

#[utoipa::path(post, path = "/api/v1/widgets", request_body = CreateWidget, tag = "widgets",
    responses((status = 201, body = Widget), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn create_widget(
    State(state): State<AppState>,
    Json(req): Json<CreateWidget>,
) -> Result<(StatusCode, Json<Widget>), ApiError> {
    if req.title.trim().is_empty() || req.target.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "title and target must not be empty".into(),
        ));
    }
    // A `datasource` tile carries its SQL in `query` (target is the datasource
    // id); every other kind carries its whole binding in `target` and must not
    // set `query`. Validate the pairing up front so a malformed tile never
    // persists.
    let query = validate_query(req.kind, req.query)?;
    let store = state.store.clone();
    let widget = blocking(move || {
        let dashboard_id = match req.dashboard_id {
            Some(id) => id,
            None => store.default_dashboard_for_site(req.site_id)?,
        };
        let widget = Widget {
            id: Uuid::new_v4(),
            dashboard_id,
            site_id: req.site_id,
            kind: req.kind,
            title: req.title,
            target: req.target,
            query,
            created_at: Utc::now(),
        };
        store.create_widget(&widget)?;
        Ok(widget)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(widget)))
}

/// Enforce the `query`/`kind` pairing: a `datasource` tile requires non-empty
/// SQL; every other kind must omit it. Returns the SQL to persist (`None` for
/// non-datasource kinds).
fn validate_query(kind: WidgetKind, query: Option<String>) -> Result<Option<String>, ApiError> {
    match kind {
        WidgetKind::Datasource => match query {
            Some(sql) if !sql.trim().is_empty() => Ok(Some(sql)),
            _ => Err(ApiError::BadRequest(
                "a datasource widget requires a non-empty `query` (native SQL)".into(),
            )),
        },
        _ if query.is_some() => Err(ApiError::BadRequest(
            "`query` is only valid for a datasource widget".into(),
        )),
        _ => Ok(None),
    }
}
