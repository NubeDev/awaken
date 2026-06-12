//! POST /api/v1/sites — validate slugs/tags, persist, return the site.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, Site, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSite {
    pub org: String,
    pub slug: String,
    pub display_name: String,
    #[serde(default)]
    pub tags: TagSet,
}

#[utoipa::path(post, path = "/api/v1/sites", request_body = CreateSite, tag = "sites",
    responses((status = 201, body = Site), (status = 400, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_site(
    State(state): State<AppState>,
    Json(req): Json<CreateSite>,
) -> Result<(StatusCode, Json<Site>), ApiError> {
    validate_slug(&req.org)?;
    validate_slug(&req.slug)?;
    req.tags.validate()?;
    let site = Site {
        id: Uuid::new_v4(),
        org: req.org,
        slug: req.slug,
        display_name: req.display_name,
        tags: req.tags,
        created_at: Utc::now(),
    };
    let stored = site.clone();
    blocking(move || Ok(state.store.create_site(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(site)))
}
