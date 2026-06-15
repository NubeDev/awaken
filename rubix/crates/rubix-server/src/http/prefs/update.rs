//! `PATCH /prefs` — update the requesting principal's display preferences.
//!
//! The partial update is merged onto the principal's current preferences (a `GET`
//! would return the same base), then written through the WS-05 gate as the
//! principal's `kind:"prefs"` record — created on first write, replaced
//! thereafter — so the change is audited like every other mutation. The merged
//! preferences are returned.

use axum::Json;
use axum::extract::State;
use rubix_gate::{Change, Command, apply, read_record_on_session};
use serde_json::Value;

use crate::auth::Authenticated;
use crate::dto::prefs::{PreferencesDto, UpdatePreferencesRequest, PREFS_KIND};
use crate::error::{ApiError, ApiResult};
use crate::http::records::capability::RECORD_WRITE;
use crate::http::records::create::map_gate_error;

use super::prefs_id;
use super::read::load_prefs;
use crate::state::AppState;

/// Merge the update onto the principal's preferences and persist through the gate.
pub async fn patch_prefs_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<UpdatePreferencesRequest>,
) -> ApiResult<Json<PreferencesDto>> {
    let current = load_prefs(&auth).await?;
    let merged = body
        .merge_onto(current)
        .map_err(ApiError::BadRequest)?;

    let id = prefs_id(auth.principal.subject.as_str());
    let content = prefs_content(&merged)?;

    // Create the record on first write, replace it thereafter — the gate has no
    // upsert verb, so the choice is made from whether the record already exists.
    let exists = read_record_on_session(&auth.session, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some();
    let change = if exists {
        Change::Update(content)
    } else {
        Change::Create(content)
    };

    let command = Command::new(auth.principal.clone(), RECORD_WRITE, id, change);
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_gate_error)?;

    Ok(Json(merged.into()))
}

/// Serialise preferences into record content tagged with the `prefs` kind.
fn prefs_content(prefs: &rubix_prefs::Preferences) -> Result<Value, ApiError> {
    let mut content = serde_json::to_value(prefs)
        .map_err(|e| ApiError::Internal(format!("serialise preferences: {e}")))?;
    if let Value::Object(map) = &mut content {
        map.insert("kind".to_owned(), Value::String(PREFS_KIND.to_owned()));
    }
    Ok(content)
}
