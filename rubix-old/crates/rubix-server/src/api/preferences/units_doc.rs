//! The `GET /api/v1/units` payload: the closed `Quantity`/`Unit` registry
//! serialised verbatim. A client uses it to populate unit pickers and to know,
//! per quantity, which units are accepted on the wire and which is canonical.

use serde::Serialize;
use utoipa::ToSchema;

/// The closed unit registry, one entry per quantity.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct UnitsDocument {
    /// Per-quantity definitions, in `Quantity::ALL` order (stable).
    pub quantities: Vec<QuantityEntry>,
}

/// One row of [`UnitsDocument::quantities`].
#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct QuantityEntry {
    /// Wire identifier (e.g. `"temperature"`).
    pub quantity: String,
    /// Canonical SI unit identifier (e.g. `"celsius"`).
    pub canonical: String,
    /// Every unit accepted on the wire for this quantity.
    pub allowed: Vec<String>,
}
