//! `Theme` — `light` / `dark` / `system`. User-only (no org fallback)
//! per the SCOPE Decisions block.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of UI theme choices. User-only field — the org-layer
/// Preferences DTO omits `theme` entirely. *Revisit trigger:* a
/// consumer needs org-enforced theming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light UI palette.
    Light,
    /// Dark UI palette.
    Dark,
    /// Follow the operating system's current preference.
    System,
}
