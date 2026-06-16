//! `GET /prefs` — the requesting principal's display preferences.
//!
//! A read on the WS-03 scoped session: the principal's own `kind:"prefs"` record,
//! or the canonical defaults when it has set none. Defaults (metric, ISO-8601,
//! UTC) mean a fresh principal always gets a usable preferences object without a
//! write first.

use axum::Json;
use rubix_gate::read_record_on_session;
use rubix_prefs::Preferences;

use crate::auth::Authenticated;
use crate::dto::prefs::PreferencesDto;
use crate::error::{ApiError, ApiResult};

use super::prefs_id;

/// Return the principal's stored preferences, or the defaults if none are set.
pub async fn get_prefs_route(auth: Authenticated) -> ApiResult<Json<PreferencesDto>> {
    let prefs = load_prefs(&auth).await?;
    Ok(Json(prefs.into()))
}

/// Load the principal's preferences from its scoped session, defaulting when the
/// record is absent.
///
/// Shared with the update route so a `PATCH` merges onto the same value a `GET`
/// would return. A stored record whose content cannot be parsed as preferences is
/// an internal error rather than a silent default — fail loud on corruption.
///
/// # Errors
/// Returns [`ApiError::Internal`] if the read fails or stored content is corrupt.
pub(crate) async fn load_prefs(auth: &Authenticated) -> Result<Preferences, ApiError> {
    let id = prefs_id(auth.principal.subject.as_str());
    let record = read_record_on_session(&auth.session, &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    match record {
        Some(record) => serde_json::from_value::<Preferences>(record.content)
            .map_err(|e| ApiError::Internal(format!("corrupt preferences record: {e}"))),
        None => Ok(Preferences::default()),
    }
}
