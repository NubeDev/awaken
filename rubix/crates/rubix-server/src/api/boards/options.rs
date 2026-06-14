//! GET /api/v1/boards/options/{source} — resolve a config field's dropdown
//! choices. A board node's config schema may tag a field with an
//! `option_source` key (e.g. `points`, `datasources`); the editor renders a
//! dropdown and fetches its choices here rather than making the operator type an
//! id. The client is agnostic to what a source means — every bit of resolution
//! logic lives in this handler, keyed by the source string.
//!
//! Scope comes from `?org=&site=` query params (the flow editor always edits a
//! board within an `{org}/{site}`). A source that needs scope and gets none is a
//! 400; a source that needs a subsystem that is not configured (no datasource
//! registry) returns an empty list, not an error, so the dropdown is simply empty.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

/// One dropdown choice: the `value` is what the node config stores, the `label`
/// is shown to the operator. Equal when a source has no friendlier label.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct OptionView {
    pub value: String,
    pub label: String,
}

impl OptionView {
    fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    /// A choice whose label is its value (no friendlier name to show).
    fn bare(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: value.clone(),
            value,
        }
    }
}

/// Editing scope for an option lookup. `org`/`site` are the `{org}/{site}` the
/// board is edited under; `datasource` narrows the `datasource_named` source to
/// one datasource's named queries.
#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct OptionsQuery {
    pub org: Option<String>,
    pub site: Option<String>,
    /// The datasource id whose named queries to list (for `datasource_named`).
    pub datasource: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/boards/options/{source}",
    params(("source" = String, Path), OptionsQuery), tag = "boards",
    responses((status = 200, body = [OptionView]), (status = 400, body = crate::error::ErrorBody)))]
pub(crate) async fn list_options(
    State(state): State<AppState>,
    Path(source): Path<String>,
    Query(q): Query<OptionsQuery>,
) -> Result<Json<Vec<OptionView>>, ApiError> {
    let options = match source.as_str() {
        "points" => points(&state, &q).await?,
        "datasources" => datasources(&state),
        "datasource_named" => datasource_named(&state, &q),
        "rules" => rules(&state, &q).await?,
        "sites" => sites(&state, &q).await?,
        _ => return Err(ApiError::NotFound("option source")),
    };
    Ok(Json(options))
}

/// Points in the editing scope, labelled `display_name` and valued by keyexpr.
async fn points(state: &AppState, q: &OptionsQuery) -> Result<Vec<OptionView>, ApiError> {
    let org = require_org(q)?;
    let store = state.store.clone();
    let site = q.site.clone();
    let rows = blocking(move || Ok(store.list_point_keyexprs(&org, site.as_deref())?)).await?;
    Ok(rows
        .into_iter()
        .map(|(keyexpr, display)| OptionView::new(keyexpr, display))
        .collect())
}

/// Registered datasource ids. Empty (not an error) when no registry is loaded,
/// so the dropdown is simply blank on a node that depends on a datasource.
fn datasources(state: &AppState) -> Vec<OptionView> {
    state
        .datasources
        .as_ref()
        .map(|reg| reg.ids().into_iter().map(OptionView::bare).collect())
        .unwrap_or_default()
}

/// Named queries for the `?datasource=` id. Empty without a registry or id.
fn datasource_named(state: &AppState, q: &OptionsQuery) -> Vec<OptionView> {
    let (Some(reg), Some(id)) = (state.datasources.as_ref(), q.datasource.as_deref()) else {
        return Vec::new();
    };
    reg.named_query_names(id)
        .into_iter()
        .map(OptionView::bare)
        .collect()
}

/// Stored rules resolvable in the editing scope (site-scoped plus org-level),
/// valued and labelled by rule name.
async fn rules(state: &AppState, q: &OptionsQuery) -> Result<Vec<OptionView>, ApiError> {
    let org = require_org(q)?;
    let store = state.store.clone();
    let site = q.site.clone();
    let records = blocking(move || {
        let site_id = match site {
            Some(slug) => Some(store.site_id_by_prefix(&format!("{org}/{slug}"))?),
            None => None,
        };
        Ok(store.list_rules(&org, site_id)?)
    })
    .await?;
    Ok(records
        .into_iter()
        .map(|r| OptionView::bare(r.name))
        .collect())
}

/// Sites in the org as `{org}/{slug}` prefixes (the value an `emit_spark` site
/// field stores), labelled by display name.
async fn sites(state: &AppState, q: &OptionsQuery) -> Result<Vec<OptionView>, ApiError> {
    let org = require_org(q)?;
    let store = state.store.clone();
    let listed = blocking(move || Ok(store.list_sites(Some(&org))?)).await?;
    Ok(listed
        .into_iter()
        .map(|s| OptionView::new(format!("{}/{}", s.org, s.slug), s.display_name))
        .collect())
}

/// The org the lookup is scoped to, or a 400 — every scoped source needs it.
fn require_org(q: &OptionsQuery) -> Result<String, ApiError> {
    q.org
        .clone()
        .ok_or(ApiError::BadRequest("option source needs an `org` scope".into()))
}
