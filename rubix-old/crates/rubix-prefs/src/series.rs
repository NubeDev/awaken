//! `SeriesEnvelope<T>` — the per-series wire shape mandated by **R8**
//! of `DOCS/user/scope/SCOPE.md` and locked by decision **D-2.1**.
//!
//! # R8 (verbatim)
//!
//! > Timeseries responses declare `quantity` and `unit` **once per
//! > series**, not per row:
//! >
//! > ```json
//! > { "series": [{
//! >   "slot": "temp_in",
//! >   "quantity": "temperature",
//! >   "unit": "fahrenheit",
//! >   "points": [[1713456000000, 72.4], [1713456060000, 72.6]]
//! > }] }
//! > ```
//!
//! `starter-server` does **not** impose this wire shape on consumer
//! endpoints — it is opt-in. Consumers wanting unit-aware serialisation
//! through `UnitsCtx` can either:
//!
//! 1. Emit a `SeriesEnvelope<f64>` directly, or
//! 2. Implement [`ToCanonicalSeries`] on their typed timeseries struct
//!    so a handler can lift it into a `SeriesEnvelope<f64>` once and
//!    let serde do the rest.
//!
//! The metadata-hoisting principle (one `{ quantity, unit }` per
//! series, not per point) is preserved by construction: `quantity` and
//! `unit` are struct-level fields, and `points` carries only `(ts,
//! value)` tuples.

use serde::{Deserialize, Serialize};
use crate::units::{Quantity, Unit};
use utoipa::ToSchema;

/// One `(timestamp_ms, value)` sample in a [`SeriesEnvelope`].
///
/// Serialises as a two-element JSON array `[ts, value]` per R8 — the
/// tuple-of-arrays shape from the SCOPE example. `serde` serialises
/// Rust tuples as JSON arrays out of the box, so this is just a named
/// alias for documentation purposes.
pub type SeriesPoint<T> = (i64, T);

/// One unit-tagged series in a timeseries response.
///
/// `quantity` and `unit` are hoisted to series scope (R8). `points`
/// carries only timestamps and values.
///
/// # Generic parameter
///
/// `T` is the value type — typically `f64` after `UnitsCtx::convert`
/// has run, but kept generic so consumers can stamp `i64` or a
/// typed-wrapper through if they prefer.
///
/// # Wire shape
///
/// ```json
/// {
///   "slot": "temp_in",
///   "quantity": "temperature",
///   "unit": "fahrenheit",
///   "points": [[1713456000000, 72.4], [1713456060000, 72.6]]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SeriesEnvelope<T>
where
    T: Clone,
{
    /// The logical name of the series within its owning response. The
    /// SCOPE example uses `"temp_in"`; consumer crates pick the
    /// vocabulary.
    pub slot: String,
    /// The physical quantity carried by every value in `points`. R8
    /// hoists this to series scope.
    pub quantity: Quantity,
    /// The unit every value in `points` is emitted in. Set to the
    /// user's preferred unit via `UnitsCtx` opt-in, or to the
    /// canonical unit when `Accept-Units: canonical`.
    pub unit: Unit,
    /// `(timestamp_ms, value)` tuples. The metadata-hoisting principle
    /// (R8) keeps `quantity` / `unit` off this vector entirely.
    ///
    /// `serde` emits Rust tuples as JSON arrays, so each point lands on
    /// the wire as `[ts_ms, value]`. The openapi schema is overridden
    /// via `#[schema(value_type = ...)]` so the generated doc shows a
    /// JSON-array shape rather than a Rust-tuple type (utoipa does not
    /// implement `ToSchema` for tuples).
    #[schema(value_type = Vec<Vec<serde_json::Value>>)]
    pub points: Vec<SeriesPoint<T>>,
}

impl<T> SeriesEnvelope<T>
where
    T: Clone,
{
    /// Trivial constructor — keeps handler call sites readable.
    pub fn new(
        slot: impl Into<String>,
        quantity: Quantity,
        unit: Unit,
        points: Vec<SeriesPoint<T>>,
    ) -> Self {
        Self {
            slot: slot.into(),
            quantity,
            unit,
            points,
        }
    }
}

/// Adapter: a typed timeseries struct on the consumer side can
/// implement this trait to surface itself as a canonical-unit
/// `SeriesEnvelope<f64>`. A handler then runs the resulting envelope
/// through `UnitsCtx::convert` (if the caller opts into preferred
/// units) before responding.
///
/// "Canonical" here means **the canonical SI unit registered for the
/// quantity** — see `crate::units::StaticRegistry::canonical`.
///
/// This trait is the opt-in surface mentioned in D-2.1 / D-2.2:
/// consumer code calls `to_canonical_series()` once at the handler
/// edge; nothing in `starter-server` walks response bodies.
pub trait ToCanonicalSeries {
    /// Render `self` as one or more canonical-unit envelopes. Returning
    /// `Vec` lets a typed multi-slot struct surface several series in
    /// one call.
    fn to_canonical_series(&self) -> Vec<SeriesEnvelope<f64>>;
}

/// Inverse adapter: build a typed timeseries struct from one or more
/// canonical-unit envelopes. Useful on the write path when accepting
/// `SeriesEnvelope<f64>` payloads and storing them as a typed value.
///
/// Implementors decide their own error type; the trait does not
/// prescribe one because validation rules are domain-specific (missing
/// slots, duplicate timestamps, quantity mismatch, …).
pub trait FromCanonicalSeries: Sized {
    /// Domain-defined construction error.
    type Error;

    /// Build `Self` from canonical-unit envelopes. The implementor is
    /// responsible for asserting each envelope's `quantity`/`unit`
    /// match its expected slot.
    fn from_canonical_series(series: &[SeriesEnvelope<f64>]) -> Result<Self, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use utoipa::PartialSchema;

    #[test]
    fn series_envelope_serialises_to_scope_r8_shape() {
        // Exact shape from SCOPE §R8.
        let env = SeriesEnvelope::<f64>::new(
            "temp_in",
            Quantity::Temperature,
            Unit::Fahrenheit,
            vec![(1_713_456_000_000, 72.4), (1_713_456_060_000, 72.6)],
        );
        let v: Value = serde_json::to_value(&env).unwrap();
        assert_eq!(
            v,
            json!({
                "slot": "temp_in",
                "quantity": "temperature",
                "unit": "fahrenheit",
                "points": [
                    [1_713_456_000_000_i64, 72.4],
                    [1_713_456_060_000_i64, 72.6],
                ],
            })
        );
    }

    #[test]
    fn series_envelope_round_trip_through_serde_json() {
        let env = SeriesEnvelope::<f64>::new(
            "speed",
            Quantity::Speed,
            Unit::MeterPerSecond,
            vec![(1, 1.5), (2, 2.5), (3, 3.5)],
        );
        let bytes = serde_json::to_vec(&env).unwrap();
        let back: SeriesEnvelope<f64> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, env);
    }

    #[test]
    fn series_envelope_metadata_hoisted_once_per_series() {
        // The hoisting principle: `quantity`/`unit` appear at series
        // scope, never on individual points. Asserted by introspecting
        // the serialised JSON.
        let env = SeriesEnvelope::<f64>::new(
            "p",
            Quantity::Pressure,
            Unit::Kilopascal,
            vec![(0, 101.3); 5],
        );
        let v = serde_json::to_value(&env).unwrap();
        assert!(v.get("quantity").is_some(), "quantity at series scope");
        assert!(v.get("unit").is_some(), "unit at series scope");
        let points = v.get("points").unwrap().as_array().unwrap();
        for p in points {
            // Each point is exactly [ts, value] — no embedded
            // quantity/unit object.
            let arr = p.as_array().expect("point is a JSON array");
            assert_eq!(
                arr.len(),
                2,
                "each point is a 2-tuple, no per-point unit metadata",
            );
        }
    }

    #[test]
    fn to_schema_emits_the_scope_r8_fields() {
        // utoipa's PartialSchema returns a RefOr<Schema>; convert to
        // a JSON Value via serde_json so we can assert on the shape.
        let schema = SeriesEnvelope::<f64>::schema();
        let v = serde_json::to_value(&schema).unwrap();
        // Drill into the object's properties — utoipa wraps under
        // {"type": "object", "properties": {...}, "required": [...]}.
        let props = v
            .pointer("/properties")
            .expect("ToSchema-generated openapi has /properties");
        for field in ["slot", "quantity", "unit", "points"] {
            assert!(
                props.get(field).is_some(),
                "openapi.json schema declares the `{field}` field",
            );
        }
        let required = v
            .pointer("/required")
            .and_then(Value::as_array)
            .expect("required is an array");
        for field in ["slot", "quantity", "unit", "points"] {
            assert!(
                required.iter().any(|r| r == field),
                "`{field}` is required in openapi.json",
            );
        }
    }

    // Compile-time check that the adapter traits are object-safe-ish
    // on the typical consumer use (`impl ToCanonicalSeries for MyT`).
    struct TempIn(Vec<(i64, f64)>);

    impl ToCanonicalSeries for TempIn {
        fn to_canonical_series(&self) -> Vec<SeriesEnvelope<f64>> {
            vec![SeriesEnvelope::new(
                "temp_in",
                Quantity::Temperature,
                Unit::Celsius,
                self.0.clone(),
            )]
        }
    }

    impl FromCanonicalSeries for TempIn {
        type Error = &'static str;

        fn from_canonical_series(series: &[SeriesEnvelope<f64>]) -> Result<Self, Self::Error> {
            let s = series
                .iter()
                .find(|s| s.slot == "temp_in")
                .ok_or("missing temp_in slot")?;
            if s.quantity != Quantity::Temperature {
                return Err("temp_in must be a temperature series");
            }
            if s.unit != Unit::Celsius {
                return Err("temp_in canonical unit is celsius");
            }
            Ok(TempIn(s.points.clone()))
        }
    }

    #[test]
    fn adapter_traits_round_trip_through_canonical_envelope() {
        let original = TempIn(vec![(10, 21.0), (20, 21.5)]);
        let envs = original.to_canonical_series();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].unit, Unit::Celsius);
        let back = TempIn::from_canonical_series(&envs).expect("round-trips");
        assert_eq!(back.0, original.0);
    }
}
