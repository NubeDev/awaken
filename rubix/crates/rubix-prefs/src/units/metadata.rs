//! `UnitMetadata` — the `{ quantity, unit }` pair declared once at the
//! tightest homogeneous scope of a typed response (per-series, or
//! inline single-value).
//!
//! # R8 (verbatim, abridged)
//!
//! > Timeseries responses declare `quantity` and `unit` **once per
//! > series**, not per row. […] Single-value reads use the inline form
//! > `{ "value": 72.4, "unit": "fahrenheit", "quantity":
//! > "temperature" }`. The rule: unit + quantity metadata are declared
//! > once at the tightest scope that covers homogeneous values.
//!
//! This type is the canonical serde representation of that
//! `{ quantity, unit }` pair. It is intentionally a thin struct (not a
//! tuple) so the openapi.json shape mirrors the SCOPE example
//! verbatim.
//!
//! The wrapper response shapes (`SeriesEnvelope<T>`, single-value
//! envelopes, …) live in **consumer-side** crates — `starter-prefs`
//! ships the platform reference DTOs in `starter_prefs::dto::series`
//! per decision **D-2.1** in `DOCS/user/scope/SCOPE.md`. `starter-spi`
//! owns only the leaf `{ quantity, unit }` pair so every crate that
//! needs to hoist unit metadata at any scope can reuse the same
//! serde-stable type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Quantity, Unit};

/// `{ quantity, unit }` metadata hoisted to the tightest homogeneous
/// scope of a typed response. See module docs for the R8 quote that
/// pins this type's role.
///
/// Field order follows the SCOPE per-series example (`quantity` before
/// `unit`) so the openapi.json field ordering matches the SCOPE
/// document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct UnitMetadata {
    /// The physical quantity carried by the values in scope. Closed
    /// enum — see [`Quantity`].
    pub quantity: Quantity,
    /// The unit the values are emitted in. Closed enum — see [`Unit`].
    /// Consumer responses set this to the user's preferred unit (via
    /// `UnitsCtx` opt-in) or to the canonical unit when
    /// `Accept-Units: canonical`.
    pub unit: Unit,
}

impl UnitMetadata {
    /// Trivial constructor; useful in handler code so the field order
    /// is obvious at the call site.
    pub const fn new(quantity: Quantity, unit: Unit) -> Self {
        Self { quantity, unit }
    }
}
