//! `NumberFormat` — the three locked grouping/decimal patterns plus
//! `Auto`.
//!
//! Wire spellings come from the SCOPE `preferences_user` column
//! comment: `"auto" | "1,234.56" | "1.234,56" | "1 234,56"`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of number-format choices. Variants locked in stage 1.
/// *Revisit trigger:* Indian grouping (`1,23,456.78`) becomes a
/// shipped requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum NumberFormat {
    /// Defer to ICU's locale-default grouping/decimal pattern.
    #[serde(rename = "auto")]
    Auto,
    /// Comma thousands, dot decimal — `1,234.56` (en-US, en-GB).
    #[serde(rename = "1,234.56")]
    CommaDot,
    /// Dot thousands, comma decimal — `1.234,56` (de-DE, es-ES).
    #[serde(rename = "1.234,56")]
    DotComma,
    /// Space thousands, comma decimal — `1 234,56` (fr-FR, ru-RU).
    #[serde(rename = "1 234,56")]
    SpaceComma,
}
