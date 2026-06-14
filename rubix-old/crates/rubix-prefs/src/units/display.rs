//! Presentation-edge helper: take a canonical-stored value, return
//! the original canonical number, the converted value in the user's
//! preferred unit, and the unit's display symbol — all in one shot.
//!
//! Aligns with SCOPE R1 ("convert at the presentation edge, never in
//! storage"). REST serialisers, CLI formatters, and React formatters
//! all call [`convert_for_display`] with the resolved per-unit pref
//! and emit the resulting [`Converted`].

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{from_canonical, Quantity, Unit, UnitError};

/// A canonical-stored value rendered for display: the original
/// canonical magnitude (so the wire can still ship the raw number if
/// it wants), the converted magnitude, and the display symbol.
///
/// Symbols are short, language-neutral glyphs (`°C`, `kWh`, `m/s`) —
/// not localised. If you need localised plural strings (`"5 kilowatt
/// hours"`), use `starter-i18n` on top of this.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Converted {
    /// The value in the canonical SI unit, exactly as stored.
    pub original: f64,
    /// The value converted to `target_unit`.
    pub value: f64,
    /// The unit the converted `value` is expressed in.
    pub unit: Unit,
    /// Short display symbol for `unit` (e.g. `°C`, `m/s`).
    pub symbol: &'static str,
}

/// Convert a canonical value to `target_unit` and bundle the result
/// with its display symbol.
pub fn convert_for_display(
    quantity: Quantity,
    canonical: f64,
    target_unit: Unit,
) -> Result<Converted, UnitError> {
    let value = from_canonical(quantity, canonical, target_unit)?;
    Ok(Converted {
        original: canonical,
        value,
        unit: target_unit,
        symbol: target_unit.symbol(),
    })
}
