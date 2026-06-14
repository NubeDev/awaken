//! `Accept-Units` content negotiation + the per-request [`UnitsCtx`] (WS-11).
//!
//! The middleware resolves the caller's preferences once per request and stashes
//! a [`UnitsCtx`] in request extensions. Handlers that emit unit-bearing values
//! call [`UnitsCtx::convert`] at serialisation time — the middleware never
//! rewrites response bodies (conversion is a handler concern, "convert at the
//! presentation edge"). The `Accept-Units` header picks the mode:
//!
//! - `preferred` (default, or a missing header) — convert into the caller's
//!   resolved unit preferences.
//! - `canonical` — emit raw canonical SI; conversion is the identity. The audit
//!   / export / "let the client convert" path.
//!
//! `Vary: Accept-Units` is appended to every response so a cache keys on the
//! negotiation axis. Identity maps as in [`super::preferences`]: user =
//! `principal.subject`, tenant = `principal.scope.org` (else the `default` org /
//! `dev` user on an auth-off edge node).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::VARY;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

use rubix_prefs::preferences::ResolvedPreferences;
use rubix_prefs::resolver::{resolve, SystemDefaults};
use rubix_prefs::units::{from_canonical, normalize_for_storage, Quantity, Unit, UnitError};

use crate::auth::Principal;
use crate::AppState;

/// Negotiated response mode for the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsMode {
    /// Convert values into the caller's resolved preferences (default).
    Preferred,
    /// Emit canonical SI verbatim; conversion is suppressed.
    Canonical,
}

impl UnitsMode {
    /// Parse a raw `Accept-Units` header value; unknown / missing → `Preferred`.
    pub fn parse(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("canonical") {
            Self::Canonical
        } else {
            Self::Preferred
        }
    }
}

/// Per-request units context, stashed in request extensions by
/// [`accept_units`]. Cheap to clone (a mode + an `Arc`).
#[derive(Debug, Clone)]
pub struct UnitsCtx {
    mode: UnitsMode,
    prefs: Arc<ResolvedPreferences>,
}

impl UnitsCtx {
    /// Construct directly — for handlers/tests that forge a context.
    pub fn new(mode: UnitsMode, prefs: Arc<ResolvedPreferences>) -> Self {
        Self { mode, prefs }
    }

    /// Negotiated mode for this request.
    pub fn mode(&self) -> UnitsMode {
        self.mode
    }

    /// The caller's resolved preferences (one DB round-trip per request).
    pub fn prefs(&self) -> &ResolvedPreferences {
        &self.prefs
    }

    /// The caller's preferred unit for `quantity`. Quantities without a prefs
    /// column yet (duration, volume, energy, power, area, angle, frequency) have
    /// no user choice, so they stay canonical — signalled by `None`.
    pub fn preferred_unit(&self, quantity: Quantity) -> Option<Unit> {
        match quantity {
            Quantity::Temperature => Some(self.prefs.temperature_unit),
            Quantity::Pressure => Some(self.prefs.pressure_unit),
            Quantity::Speed => Some(self.prefs.speed_unit),
            Quantity::Length => Some(self.prefs.length_unit),
            Quantity::Mass => Some(self.prefs.mass_unit),
            Quantity::Duration
            | Quantity::Volume
            | Quantity::Energy
            | Quantity::Power
            | Quantity::Area
            | Quantity::Angle
            | Quantity::Frequency => None,
        }
    }

    /// Convert `value` (in `source_unit`) for the response. In
    /// [`UnitsMode::Canonical`], or for a quantity with no preference column,
    /// the value is normalised to canonical and returned with the canonical
    /// unit. In [`UnitsMode::Preferred`], it is converted to the caller's
    /// preferred unit. Returns the converted numeric and the unit it is now in.
    pub fn convert(
        &self,
        quantity: Quantity,
        value: f64,
        source_unit: Unit,
    ) -> Result<(f64, Unit), UnitError> {
        let canonical = normalize_for_storage(quantity, value, source_unit)?;
        let target = match self.mode {
            UnitsMode::Canonical => None,
            UnitsMode::Preferred => self.preferred_unit(quantity),
        };
        match target {
            Some(unit) => Ok((from_canonical(quantity, canonical, unit)?, unit)),
            // Canonical mode, or no preference column: the canonical unit is the
            // registry canonical, recovered by an identity `from_canonical`.
            None => {
                let unit = canonical_unit(quantity);
                Ok((canonical, unit))
            }
        }
    }
}

impl UnitsCtx {
    /// Parse a `quantity` wire code and convert `value` for the response,
    /// interpreting `stored_unit` (the unit the column is already in; defaults to
    /// the quantity's canonical when absent). Returns the converted numeric and
    /// the wire code of the unit it is now in. `Err` carries a human-readable
    /// reason (bad quantity/unit code) for a 400.
    pub fn convert_field(
        &self,
        quantity: &str,
        stored_unit: Option<&str>,
        value: f64,
    ) -> Result<(f64, String), String> {
        let quantity: Quantity = quantity
            .parse()
            .map_err(|_| format!("unknown quantity {quantity:?}"))?;
        let source = match stored_unit {
            Some(u) => u.parse::<Unit>().map_err(|_| format!("unknown unit {u:?}"))?,
            None => canonical_unit(quantity),
        };
        let (v, unit) = self
            .convert(quantity, value, source)
            .map_err(|e| e.to_string())?;
        Ok((v, unit.as_str().to_owned()))
    }
}

/// The canonical unit for a quantity (the registry's canonical). Centralised so
/// `convert`'s canonical branch and any caller agree on the wire unit.
fn canonical_unit(quantity: Quantity) -> Unit {
    use rubix_prefs::units::{StaticRegistry, UnitRegistry};
    StaticRegistry::new()
        .get(quantity)
        .expect("StaticRegistry covers every Quantity")
        .canonical
}

/// `Accept-Units` middleware. Resolves the caller's prefs once and inserts a
/// [`UnitsCtx`]; appends `Vary: Accept-Units`. Mounted at the query/series edge.
pub async fn accept_units(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mode = request
        .headers()
        .get("accept-units")
        .and_then(|v| v.to_str().ok())
        .map(UnitsMode::parse)
        .unwrap_or(UnitsMode::Preferred);

    // Identity from the principal (auth-on) or the dev/default fallback.
    let principal = request.extensions().get::<Principal>().cloned();
    let user_id = principal
        .as_ref()
        .map(|p| p.subject.clone())
        .unwrap_or_else(|| "dev".to_owned());
    let org = principal
        .as_ref()
        .and_then(|p| p.scope.org.clone())
        .unwrap_or_else(|| "default".to_owned());

    // Resolve on the blocking pool (sync store). On any store error fall back to
    // the system defaults rather than failing the request — units are a
    // presentation concern and a query should still serve canonical values.
    let store = state.store.clone();
    let resolved = tokio::task::spawn_blocking(move || {
        let user = store.get_user_prefs(&user_id, &org).ok().flatten();
        let org_row = store.get_org_prefs(&org).ok().flatten();
        resolve(user, org_row, &SystemDefaults::starter())
    })
    .await
    .unwrap_or_else(|_| resolve(None, None, &SystemDefaults::starter()));

    let mut request = request;
    request
        .extensions_mut()
        .insert(UnitsCtx::new(mode, Arc::new(resolved)));

    let mut response = next.run(request).await;
    response
        .headers_mut()
        .append(VARY, HeaderValue::from_static("Accept-Units"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(mode: UnitsMode) -> UnitsCtx {
        UnitsCtx::new(mode, Arc::new(resolve(None, None, &SystemDefaults::starter())))
    }

    #[test]
    fn preferred_converts_to_user_unit() {
        // Defaults are metric → celsius; a canonical 25.0 °C stays 25.0 °C.
        let (v, u) = ctx(UnitsMode::Preferred)
            .convert(Quantity::Temperature, 25.0, Unit::Celsius)
            .unwrap();
        assert_eq!(u, Unit::Celsius);
        assert!((v - 25.0).abs() < 1e-9);
    }

    #[test]
    fn canonical_mode_normalises_and_stays_canonical() {
        // 32 °F → 0 °C canonical, returned in celsius regardless of prefs.
        let (v, u) = ctx(UnitsMode::Canonical)
            .convert(Quantity::Temperature, 32.0, Unit::Fahrenheit)
            .unwrap();
        assert_eq!(u, Unit::Celsius);
        assert!(v.abs() < 1e-9);
    }

    #[test]
    fn quantity_without_pref_column_stays_canonical() {
        // Energy has no prefs column; even in Preferred mode it stays canonical.
        let (_, u) = ctx(UnitsMode::Preferred)
            .convert(Quantity::Energy, 1.0, Unit::KilowattHour)
            .unwrap();
        assert_eq!(u, canonical_unit(Quantity::Energy));
    }
}
