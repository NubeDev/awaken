//! `DateFormat` — three locked patterns plus `Auto`.
//!
//! Wire spellings come straight from the SCOPE `preferences_user`
//! column comment: `"auto" | "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY"`.
//! `Auto` defers to ICU's locale-default short-date pattern (per R3
//! the resolver, not this DTO, picks the concrete pattern; once
//! resolved the field carries one of the three explicit variants).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of date-format choices. Variants locked in stage 1.
/// *Revisit trigger:* a locale ships that none of these three formats
/// fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DateFormat {
    /// Defer to ICU's locale-default short-date pattern.
    #[serde(rename = "auto")]
    Auto,
    /// ISO 8601 — `YYYY-MM-DD`.
    #[serde(rename = "YYYY-MM-DD")]
    IsoYMD,
    /// Day / month / year — `DD/MM/YYYY`.
    #[serde(rename = "DD/MM/YYYY")]
    DmySlash,
    /// Month / day / year — `MM/DD/YYYY`.
    #[serde(rename = "MM/DD/YYYY")]
    MdySlash,
}
