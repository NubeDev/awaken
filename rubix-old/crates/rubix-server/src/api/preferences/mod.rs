//! Units & datetime preferences (WS-11): the per-user and per-org preference
//! surface, plus the closed unit registry document.
//!
//! - `GET /api/v1/me/preferences` — the caller's fully-resolved preferences
//!   (`user → org → system default`, with `"auto"` derivation collapsed). Any
//!   authenticated caller; on an auth-off edge node it returns the system
//!   defaults so the dev UI still renders.
//! - `PATCH /api/v1/me/preferences` — partial update of the caller's user-layer
//!   row. A missing key leaves a field unchanged; an explicit JSON `null`
//!   reverts it to inherit. Returns the re-resolved preferences.
//! - `GET/PATCH /api/v1/orgs/{org}/preferences` — the org-layer row; admin-gated
//!   (the org default every member inherits).
//! - `GET /api/v1/units` — the closed `Quantity`/`Unit` registry, public.
//!
//! Identity maps to rubix's model: the user id is `principal.subject`, the
//! tenant is `principal.scope.org` (rubix's org = starter's "workspace"). The
//! store is route-pinned on that org, not RLS-bound — a caller only ever reads
//! or writes prefs in the org their scope covers. See WS-11.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Map, Value as JsonValue};

use rubix_prefs::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use rubix_prefs::resolver::{
    resolve, OrgPrefsRow, StringPref, SystemDefaults, UnitPref, UserPrefsRow,
};
use rubix_prefs::units::{Quantity, StaticRegistry, Unit, UnitRegistry};

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

mod units_doc;
pub(crate) use units_doc::UnitsDocument;

/// The tenant a caller's preferences are pinned to. The org from the principal's
/// scope; on an auth-off edge node (no principal) or a global principal there is
/// no org, so prefs live under this shared sentinel — enough to make a
/// single-tenant edge node's prefs persist without inventing a tenant.
const DEFAULT_ORG: &str = "default";

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/me/preferences",
            get(get_me_preferences).patch(patch_me_preferences),
        )
        .route(
            "/api/v1/orgs/{org}/preferences",
            get(get_org_preferences).patch(patch_org_preferences),
        )
        .route("/api/v1/units", get(get_units))
}

/// The org a `/me` request resolves against: the principal's scope org, or the
/// shared default when global / auth-off.
fn me_org(principal: &RequestPrincipal) -> String {
    principal
        .0
        .as_ref()
        .and_then(|p| p.scope.org.clone())
        .unwrap_or_else(|| DEFAULT_ORG.to_owned())
}

/// The user id a `/me` request keys on: the principal's subject, or a dev
/// sentinel when auth is off (matching `whoami`'s synthetic dev identity).
fn me_user(principal: &RequestPrincipal) -> String {
    principal
        .0
        .as_ref()
        .map(|p| p.subject.clone())
        .unwrap_or_else(|| "dev".to_owned())
}

#[utoipa::path(get, path = "/api/v1/me/preferences", tag = "preferences",
    security(("bearer" = [])),
    responses((status = 200, body = ResolvedPreferences)))]
pub(crate) async fn get_me_preferences(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<ResolvedPreferences>, ApiError> {
    let (user_id, org) = (me_user(&principal), me_org(&principal));
    let resolved = blocking(move || {
        let user = state.store.get_user_prefs(&user_id, &org)?;
        let org_row = state.store.get_org_prefs(&org)?;
        Ok(resolve(user, org_row, &SystemDefaults::starter()))
    })
    .await?;
    Ok(Json(resolved))
}

#[utoipa::path(patch, path = "/api/v1/me/preferences", request_body = serde_json::Value,
    tag = "preferences", security(("bearer" = [])),
    responses((status = 200, body = ResolvedPreferences), (status = 400, body = ErrorBody)))]
pub(crate) async fn patch_me_preferences(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(patch): Json<JsonValue>,
) -> Result<Json<ResolvedPreferences>, ApiError> {
    let (user_id, org) = (me_user(&principal), me_org(&principal));
    let patch = as_object(patch)?;
    let resolved = blocking(move || {
        let mut row = state.store.get_user_prefs(&user_id, &org)?.unwrap_or_default();
        apply_user_patch(&mut row, &patch).map_err(ApiError::BadRequest)?;
        state.store.upsert_user_prefs(&user_id, &org, &row)?;
        let org_row = state.store.get_org_prefs(&org)?;
        Ok(resolve(Some(row), org_row, &SystemDefaults::starter()))
    })
    .await?;
    Ok(Json(resolved))
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/preferences", tag = "preferences",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = ResolvedPreferences), (status = 403, body = ErrorBody)))]
pub(crate) async fn get_org_preferences(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
) -> Result<Json<ResolvedPreferences>, ApiError> {
    principal.require_admin(&org)?;
    let resolved = blocking(move || {
        let org_row = state.store.get_org_prefs(&org)?;
        // Org view: resolve the org layer against the system default, no user
        // layer (this is the org's own baseline, not any member's view).
        Ok(resolve(None, org_row, &SystemDefaults::starter()))
    })
    .await?;
    Ok(Json(resolved))
}

#[utoipa::path(patch, path = "/api/v1/orgs/{org}/preferences", request_body = serde_json::Value,
    tag = "preferences", params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = ResolvedPreferences), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn patch_org_preferences(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(patch): Json<JsonValue>,
) -> Result<Json<ResolvedPreferences>, ApiError> {
    principal.require_admin(&org)?;
    let patch = as_object(patch)?;
    let resolved = blocking(move || {
        let mut row = state.store.get_org_prefs(&org)?.unwrap_or_default();
        apply_org_patch(&mut row, &patch).map_err(ApiError::BadRequest)?;
        state.store.upsert_org_prefs(&org, &row)?;
        Ok(resolve(None, Some(row), &SystemDefaults::starter()))
    })
    .await?;
    Ok(Json(resolved))
}

#[utoipa::path(get, path = "/api/v1/units", tag = "preferences",
    responses((status = 200, body = UnitsDocument)))]
pub(crate) async fn get_units() -> Json<UnitsDocument> {
    let registry = StaticRegistry::new();
    let quantities = Quantity::ALL
        .iter()
        .map(|q| {
            let def = registry.get(*q).expect("StaticRegistry covers all Quantity");
            units_doc::QuantityEntry {
                quantity: q.as_str().to_owned(),
                canonical: def.canonical.as_str().to_owned(),
                allowed: def.allowed_units.iter().map(|u| u.as_str().to_owned()).collect(),
            }
        })
        .collect();
    Json(UnitsDocument { quantities })
}

// ---------------------------------------------------------------------
// PATCH plumbing — absent key = leave unchanged, JSON null = revert to inherit.
// ---------------------------------------------------------------------

fn as_object(value: JsonValue) -> Result<Map<String, JsonValue>, ApiError> {
    match value {
        JsonValue::Object(map) => Ok(map),
        JsonValue::Null => Ok(Map::new()),
        _ => Err(ApiError::BadRequest("body must be a JSON object".into())),
    }
}

fn apply_user_patch(row: &mut UserPrefsRow, patch: &Map<String, JsonValue>) -> Result<(), String> {
    for (key, value) in patch {
        match key.as_str() {
            "timezone" => row.timezone = parse_opt_string_pref(value, key)?,
            "locale" => row.locale = parse_opt_string(value, key)?,
            "language" => row.language = parse_opt_string(value, key)?,
            "unit_system" => row.unit_system = parse_opt_enum::<UnitSystem>(value, key)?,
            "temperature_unit" => row.temperature_unit = parse_opt_unit(value, key)?,
            "pressure_unit" => row.pressure_unit = parse_opt_unit(value, key)?,
            "speed_unit" => row.speed_unit = parse_opt_unit(value, key)?,
            "length_unit" => row.length_unit = parse_opt_unit(value, key)?,
            "mass_unit" => row.mass_unit = parse_opt_unit(value, key)?,
            "date_format" => row.date_format = parse_opt_enum::<DateFormat>(value, key)?,
            "time_format" => row.time_format = parse_opt_enum::<TimeFormat>(value, key)?,
            "week_start" => row.week_start = parse_opt_enum::<WeekStart>(value, key)?,
            "number_format" => row.number_format = parse_opt_enum::<NumberFormat>(value, key)?,
            "currency" => row.currency = parse_opt_string_pref(value, key)?,
            "theme" => row.theme = parse_opt_enum::<Theme>(value, key)?,
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(())
}

fn apply_org_patch(row: &mut OrgPrefsRow, patch: &Map<String, JsonValue>) -> Result<(), String> {
    for (key, value) in patch {
        match key.as_str() {
            "timezone" => row.timezone = parse_opt_string_pref(value, key)?,
            "locale" => row.locale = parse_opt_string(value, key)?,
            "language" => row.language = parse_opt_string(value, key)?,
            "unit_system" => row.unit_system = parse_opt_enum::<UnitSystem>(value, key)?,
            "temperature_unit" => row.temperature_unit = parse_opt_unit(value, key)?,
            "pressure_unit" => row.pressure_unit = parse_opt_unit(value, key)?,
            "speed_unit" => row.speed_unit = parse_opt_unit(value, key)?,
            "length_unit" => row.length_unit = parse_opt_unit(value, key)?,
            "mass_unit" => row.mass_unit = parse_opt_unit(value, key)?,
            "date_format" => row.date_format = parse_opt_enum::<DateFormat>(value, key)?,
            "time_format" => row.time_format = parse_opt_enum::<TimeFormat>(value, key)?,
            "week_start" => row.week_start = parse_opt_enum::<WeekStart>(value, key)?,
            "number_format" => row.number_format = parse_opt_enum::<NumberFormat>(value, key)?,
            "currency" => row.currency = parse_opt_string_pref(value, key)?,
            "theme" => return Err("org preferences do not carry `theme`".into()),
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(())
}

fn parse_opt_string(value: &JsonValue, key: &str) -> Result<Option<String>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn parse_opt_string_pref(value: &JsonValue, key: &str) -> Result<Option<StringPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(StringPref::parse(s))),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn parse_opt_enum<T: for<'de> serde::Deserialize<'de>>(
    value: &JsonValue,
    key: &str,
) -> Result<Option<T>, String> {
    match value {
        JsonValue::Null => Ok(None),
        other => serde_json::from_value::<T>(other.clone())
            .map(Some)
            .map_err(|e| format!("{key:?}: {e}")),
    }
}

fn parse_opt_unit(value: &JsonValue, key: &str) -> Result<Option<UnitPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) if s == "auto" => Ok(Some(UnitPref::Auto)),
        JsonValue::String(_) => serde_json::from_value::<Unit>(value.clone())
            .map(|u| Some(UnitPref::Explicit(u)))
            .map_err(|e| format!("{key:?}: {e}")),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}
