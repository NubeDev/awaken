//! PATCH /api/v1/widgets/{id} — update a tile's presentation settings (grid
//! layout + chart config). Identity, target, and query are immutable (a rebind
//! is delete-and-recreate); only `settings` is mutable, so the builder can
//! drag/resize a tile and pick its chart type without re-pinning.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::{Widget, WidgetSettings};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchWidget {
    /// New presentation settings. An explicit `null` clears them back to the
    /// default rendering; omitting the field leaves them unchanged.
    #[serde(default, deserialize_with = "double_option")]
    pub settings: Option<Option<WidgetSettings>>,
}

#[utoipa::path(patch, path = "/api/v1/widgets/{id}", params(("id" = Uuid, Path)),
    request_body = PatchWidget, tag = "widgets",
    responses((status = 200, body = Widget), (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_widget(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchWidget>,
) -> Result<Json<Widget>, ApiError> {
    let updated = blocking(move || {
        // `settings` absent → leave the row as-is (re-read it); present →
        // replace (a `null` clears, an object sets).
        match req.settings {
            None => Ok(state.store.get_widget(id)?),
            Some(settings) => Ok(state.store.update_widget_settings(id, settings.as_ref())?),
        }
    })
    .await?;
    Ok(Json(updated))
}

/// Distinguish "field omitted" (`None`) from "field present and null"
/// (`Some(None)`) so a PATCH can either leave settings untouched or clear them.
fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Ok(Some(Option::deserialize(de)?))
}
