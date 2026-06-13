//! User and org preferences — the wire surface.
//!
//! This module owns the closed enums for per-preferences fields
//! (`UnitSystem`, `Theme`, `DateFormat`, `TimeFormat`, `WeekStart`,
//! `NumberFormat`) and the two DTOs the HTTP surface speaks in:
//!
//! - [`ResolvedPreferences`] — the **resolved** view a request handler
//!   gets after the Phase 1 resolver has walked
//!   user → org → system → ICU and produced a concrete value for every
//!   field. **No `Option`, no `"auto"` string.** Per R3 of
//!   `DOCS/user/scope/SCOPE.md` (Hard rules) the resolver is the only
//!   thing that produces a `ResolvedPreferences`; downstream code never
//!   has to second-guess a `None`.
//!
//! - [`PreferencesPatch`] — the wire shape of a `PATCH` body. Every
//!   field is `Option<T>`; the resolver interprets `None` as "leave
//!   alone" and `Some(null)` as "revert to inherit" (this distinction
//!   is encoded in the route layer in Phase 1; this crate just carries
//!   the shape).
//!
//! Enum variant wire spellings match the SCOPE column-comment strings
//! byte-for-byte — see `DOCS/user/scope/SCOPE.md` "Preferences model"
//! section. Where snake_case lines up (e.g. `metric`, `light`,
//! `monday`) we rely on `#[serde(rename_all = "snake_case")]`; where it
//! does not (e.g. `"YYYY-MM-DD"`, `"24h"`, `"1,234.56"`) the variants
//! carry an explicit `#[serde(rename = "…")]`.
//!
//! Module layout follows the workspace "one responsibility per file"
//! rule: each public type lives in its own file and is re-exported
//! here.

mod date_format;
mod number_format;
mod patch;
mod resolved;
mod theme;
mod time_format;
mod unit_system;
mod week_start;

pub use date_format::DateFormat;
pub use number_format::NumberFormat;
pub use patch::PreferencesPatch;
pub use resolved::ResolvedPreferences;
pub use theme::Theme;
pub use time_format::TimeFormat;
pub use unit_system::UnitSystem;
pub use week_start::WeekStart;

#[cfg(test)]
mod tests;
