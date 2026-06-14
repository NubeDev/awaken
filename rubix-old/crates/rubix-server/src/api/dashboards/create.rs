//! POST /api/v1/dashboards — create a named board, org-overview or site-scoped.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, validate_variables, Dashboard, Variable};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::authorize_dashboard_write;
use crate::api::audit::record::{actor_of, Recorder};
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDashboard {
    /// Owning org namespace.
    pub org: String,
    /// The single site this board is for; omit for an org overview.
    #[serde(default)]
    pub site_id: Option<Uuid>,
    pub slug: String,
    pub title: String,
    /// Dashboard variables (docs/design/variables-and-templating.md §1). Empty /
    /// omitted for a board with no parameterisation.
    #[serde(default)]
    pub variables: Vec<Variable>,
}

#[utoipa::path(post, path = "/api/v1/dashboards", request_body = CreateDashboard, tag = "dashboards",
    security(("bearer" = [])),
    responses((status = 201, body = Dashboard), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_dashboard(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<CreateDashboard>,
) -> Result<(StatusCode, Json<Dashboard>), ApiError> {
    validate_slug(&req.org)?;
    validate_slug(&req.slug)?;
    if req.title.trim().is_empty() {
        return Err(ApiError::BadRequest("title must not be empty".into()));
    }
    validate_variables(&req.variables).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    authorize_dashboard_write(&principal, &state.store, &req.org, req.site_id)?;

    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        org: req.org,
        site_id: req.site_id,
        slug: req.slug,
        title: req.title,
        variables: req.variables,
        created_at: Utc::now(),
    };
    let stored = dashboard.clone();
    let actor = actor_of(&principal);
    blocking(move || {
        state.store.create_dashboard(&stored)?;
        // Record the creation next to the mutation it describes (the snapshot is the
        // new row); the recorder is the sole ledger write path.
        Recorder::new(actor, stored.org.clone(), stored.site_id).create(
            &state.store,
            "dashboard",
            stored.id,
            serde_json::to_value(&stored).map_err(anyhow::Error::from)?,
        )?;
        Ok(())
    })
    .await?;
    Ok((StatusCode::CREATED, Json(dashboard)))
}
