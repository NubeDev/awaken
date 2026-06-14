//! `TimeFormat` — 24-hour, 12-hour, or `Auto`.
//!
//! Wire spellings come from the SCOPE `preferences_user` column
//! comment: `"auto" | "24h" | "12h"`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of clock formats. Variants locked in stage 1.
/// *Revisit trigger:* none expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum TimeFormat {
    /// Defer to ICU's locale-default time pattern.
    #[serde(rename = "auto")]
    Auto,
    /// 24-hour clock — `13:42`.
    #[serde(rename = "24h")]
    H24,
    /// 12-hour clock with AM/PM — `1:42 PM`.
    #[serde(rename = "12h")]
    H12,
}
