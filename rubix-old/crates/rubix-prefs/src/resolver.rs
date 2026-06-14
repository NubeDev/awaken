//! Three-layer preference resolver.
//!
//! Owns: SCOPE.md **R3 — Three-layer resolution: user → org →
//! default** and the `"auto"` derivation rules described in the same
//! section.
//!
//! The resolver is a **pure function**: no I/O, no clock, no DB
//! handle. It takes one optional user row, one optional org row, and
//! the system defaults, and produces a fully-concrete
//! [`ResolvedPreferences`] (no `Option`, no `"auto"` sentinel) per the
//! Phase 0 D-U0.3 + ResolvedPreferences invariant in
//! `DOCS/user/scope/SCOPE.md`.
//!
//! Per-field resolution rule (R3 verbatim): "for each individual
//! column the resolver picks the first non-null layer
//! (user → org → default). It does **not** cross-reference other
//! columns. The cross-column interaction (`unit_system` vs
//! `temperature_unit`) is handled inside the `"auto"` resolution rule
//! below — never by overlaying one column on top of another."
//!
//! `"auto"` derivation order for per-unit fields (R3 verbatim):
//! ```text
//! explicit value at any layer
//!   → unit_system at the same/closer layer (metric / imperial table)
//!   → locale-derived default (ICU, where one exists)
//!   → hardcoded system default
//! ```
//!
//! `currency: "auto"` derives from `locale` via `iso_currency`'s
//! country → currency mapping (e.g. `en-AU` → `AUD`). Display-only
//! fields with an `Auto` variant (`DateFormat`, `TimeFormat`,
//! `WeekStart`, `NumberFormat`) fall back to the hardcoded system
//! default at this layer; ICU-driven locale defaults land in a later
//! stage when the i18n crate's ICU integration is wired in.

use crate::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use crate::units::{Quantity, Unit};

// ---------------------------------------------------------------------
// Row types (storage-shaped, all-nullable).
//
// These mirror the `starter_prefs_user` / `starter_prefs_org` schemas
// in SCOPE.md "Preferences model". The store layer (stage 6) maps
// rows from sqlx into these structs; the resolver consumes them.
// Per-unit fields are modelled as `Option<UnitPref>` where `None`
// means SQL NULL ("inherit") and `Some(UnitPref::Auto)` means the row
// explicitly carries the `"auto"` sentinel ("derive at resolve
// time"). Same shape for `currency` / `timezone`, which are stored as
// free strings but accept `"auto"` per the SCOPE column comments.
// ---------------------------------------------------------------------

/// A per-unit preference value as stored. `None` (the outer
/// `Option<UnitPref>` on a row) is SQL NULL; `Some(Auto)` is the
/// explicit `"auto"` sentinel; `Some(Explicit(u))` is a concrete
/// `Unit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitPref {
    /// `"auto"` sentinel — derive at resolve time per R3.
    Auto,
    /// Explicit unit chosen by the user / org.
    Explicit(Unit),
}

/// A string-shaped preference value (currency, timezone) that
/// supports the `"auto"` sentinel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StringPref {
    /// `"auto"` sentinel — derive at resolve time per R3.
    Auto,
    /// Explicit value.
    Explicit(String),
}

impl StringPref {
    /// Parse a raw column string: `"auto"` → [`Self::Auto`], anything
    /// else → [`Self::Explicit`]. Useful for the store layer.
    pub fn parse(s: &str) -> Self {
        if s == "auto" {
            Self::Auto
        } else {
            Self::Explicit(s.to_owned())
        }
    }
}

/// User-layer preference row, all-nullable. Mirrors
/// `starter_prefs_user` in SCOPE.md "Preferences model".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserPrefsRow {
    /// IANA timezone or `"auto"`.
    pub timezone: Option<StringPref>,
    /// BCP-47 locale tag.
    pub locale: Option<String>,
    /// BCP-47 language subtag.
    pub language: Option<String>,
    /// `metric` / `imperial`.
    pub unit_system: Option<UnitSystem>,
    /// Per-unit overrides — see [`UnitPref`].
    pub temperature_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub pressure_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub speed_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub length_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub mass_unit: Option<UnitPref>,
    /// Display fields with an `Auto` variant baked in.
    pub date_format: Option<DateFormat>,
    /// Display fields with an `Auto` variant baked in.
    pub time_format: Option<TimeFormat>,
    /// Display fields with an `Auto` variant baked in.
    pub week_start: Option<WeekStart>,
    /// Display fields with an `Auto` variant baked in.
    pub number_format: Option<NumberFormat>,
    /// ISO 4217 code or `"auto"`.
    pub currency: Option<StringPref>,
    /// User-only field with no org counterpart per the SCOPE Decisions
    /// block.
    pub theme: Option<Theme>,
}

/// Org-layer preference row. Same shape as [`UserPrefsRow`] minus
/// `theme` (org-layer omits `theme` per the SCOPE Decisions block).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrgPrefsRow {
    /// IANA timezone or `"auto"`.
    pub timezone: Option<StringPref>,
    /// BCP-47 locale tag.
    pub locale: Option<String>,
    /// BCP-47 language subtag.
    pub language: Option<String>,
    /// `metric` / `imperial`.
    pub unit_system: Option<UnitSystem>,
    /// Per-unit overrides — see [`UnitPref`].
    pub temperature_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub pressure_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub speed_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub length_unit: Option<UnitPref>,
    /// Per-unit overrides — see [`UnitPref`].
    pub mass_unit: Option<UnitPref>,
    /// Display fields with an `Auto` variant baked in.
    pub date_format: Option<DateFormat>,
    /// Display fields with an `Auto` variant baked in.
    pub time_format: Option<TimeFormat>,
    /// Display fields with an `Auto` variant baked in.
    pub week_start: Option<WeekStart>,
    /// Display fields with an `Auto` variant baked in.
    pub number_format: Option<NumberFormat>,
    /// ISO 4217 code or `"auto"`.
    pub currency: Option<StringPref>,
}

/// Hardcoded system defaults — the last-resort layer per R3. Concrete
/// values only (no `Option`, no `Auto`). Per SCOPE.md "Hard rules"
/// R3: "the default is hardcoded in `starter-spi` (`en-US`, `UTC`,
/// metric, ISO date, 24h, `1,234.56` number format, `system`
/// theme)". This struct is the in-crate value carrier; the
/// [`SystemDefaults::starter()`] constructor returns that exact
/// configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemDefaults {
    /// IANA timezone (e.g. `"UTC"`).
    pub timezone: String,
    /// BCP-47 locale tag (e.g. `"en-US"`).
    pub locale: String,
    /// BCP-47 language subtag (e.g. `"en"`).
    pub language: String,
    /// `metric` / `imperial`.
    pub unit_system: UnitSystem,
    /// Concrete temperature unit.
    pub temperature_unit: Unit,
    /// Concrete pressure unit.
    pub pressure_unit: Unit,
    /// Concrete speed unit.
    pub speed_unit: Unit,
    /// Concrete length unit.
    pub length_unit: Unit,
    /// Concrete mass unit.
    pub mass_unit: Unit,
    /// Concrete date format (never `Auto`).
    pub date_format: DateFormat,
    /// Concrete time format (never `Auto`).
    pub time_format: TimeFormat,
    /// Concrete week-start (never `Auto`).
    pub week_start: WeekStart,
    /// Concrete number format (never `Auto`).
    pub number_format: NumberFormat,
    /// ISO 4217 code (never `"auto"`).
    pub currency: String,
    /// UI theme.
    pub theme: Theme,
}

impl SystemDefaults {
    /// The starter-shipped defaults — `en-US`, `UTC`, metric, ISO
    /// date, 24h, `1,234.56`, `system` theme.
    pub fn starter() -> Self {
        Self {
            timezone: "UTC".to_owned(),
            locale: "en-US".to_owned(),
            language: "en".to_owned(),
            unit_system: UnitSystem::Metric,
            temperature_unit: Unit::Celsius,
            pressure_unit: Unit::Kilopascal,
            speed_unit: Unit::MeterPerSecond,
            length_unit: Unit::Meter,
            mass_unit: Unit::Kilogram,
            date_format: DateFormat::IsoYMD,
            time_format: TimeFormat::H24,
            week_start: WeekStart::Monday,
            number_format: NumberFormat::CommaDot,
            currency: "USD".to_owned(),
            theme: Theme::System,
        }
    }
}

impl Default for SystemDefaults {
    fn default() -> Self {
        Self::starter()
    }
}

// ---------------------------------------------------------------------
// Public entry point.
// ---------------------------------------------------------------------

/// Resolve preferences across the three layers per R3.
///
/// Pure function, no I/O, deterministic. The first non-null layer
/// (user → org → default) wins **per column**; cross-column
/// interactions (`unit_system` driving an `Auto` per-unit field) are
/// confined to the `"auto"` derivation rule below and never overlay
/// one column on top of another at the layer-merge step.
pub fn resolve(
    user: Option<UserPrefsRow>,
    org: Option<OrgPrefsRow>,
    default: &SystemDefaults,
) -> ResolvedPreferences {
    // -- Scalar string fields (no Auto sentinel): timezone, locale,
    // -- language. `timezone` does support `"auto"` per the SCOPE
    // -- column comment, but R3 has no derivation order for it
    // -- (locale-driven tz derivation is out of scope here — there's
    // -- no 1:1 locale→tz mapping). `"auto"` on timezone collapses to
    // -- the system default.
    let locale = first_string(
        user.as_ref().and_then(|u| u.locale.clone()),
        org.as_ref().and_then(|o| o.locale.clone()),
        &default.locale,
    );
    let language = first_string(
        user.as_ref().and_then(|u| u.language.clone()),
        org.as_ref().and_then(|o| o.language.clone()),
        &default.language,
    );
    let timezone = resolve_string_pref(
        user.as_ref().and_then(|u| u.timezone.clone()),
        org.as_ref().and_then(|o| o.timezone.clone()),
        || None, // no locale → timezone derivation in v1
        &default.timezone,
    );

    // -- unit_system: closed enum, no Auto variant. First non-null
    // -- wins.
    let unit_system = first_copy(
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
    );

    // -- Per-unit fields: apply the R3 "auto" derivation order.
    let temperature_unit = resolve_unit(
        Quantity::Temperature,
        user.as_ref().and_then(|u| u.temperature_unit),
        org.as_ref().and_then(|o| o.temperature_unit),
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
        default.temperature_unit,
    );
    let pressure_unit = resolve_unit(
        Quantity::Pressure,
        user.as_ref().and_then(|u| u.pressure_unit),
        org.as_ref().and_then(|o| o.pressure_unit),
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
        default.pressure_unit,
    );
    let speed_unit = resolve_unit(
        Quantity::Speed,
        user.as_ref().and_then(|u| u.speed_unit),
        org.as_ref().and_then(|o| o.speed_unit),
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
        default.speed_unit,
    );
    let length_unit = resolve_unit(
        Quantity::Length,
        user.as_ref().and_then(|u| u.length_unit),
        org.as_ref().and_then(|o| o.length_unit),
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
        default.length_unit,
    );
    let mass_unit = resolve_unit(
        Quantity::Mass,
        user.as_ref().and_then(|u| u.mass_unit),
        org.as_ref().and_then(|o| o.mass_unit),
        user.as_ref().and_then(|u| u.unit_system),
        org.as_ref().and_then(|o| o.unit_system),
        default.unit_system,
        default.mass_unit,
    );

    // -- Display fields with Auto variants. Until the ICU integration
    // -- lands the Auto cases fall back to the hardcoded default; the
    // -- per-field signature is identical so future stages can swap
    // -- the body without touching callers.
    let date_format = resolve_display(
        user.as_ref().and_then(|u| u.date_format),
        org.as_ref().and_then(|o| o.date_format),
        DateFormat::Auto,
        default.date_format,
    );
    let time_format = resolve_display(
        user.as_ref().and_then(|u| u.time_format),
        org.as_ref().and_then(|o| o.time_format),
        TimeFormat::Auto,
        default.time_format,
    );
    let week_start = resolve_display(
        user.as_ref().and_then(|u| u.week_start),
        org.as_ref().and_then(|o| o.week_start),
        WeekStart::Auto,
        default.week_start,
    );
    let number_format = resolve_display(
        user.as_ref().and_then(|u| u.number_format),
        org.as_ref().and_then(|o| o.number_format),
        NumberFormat::Auto,
        default.number_format,
    );

    // -- currency: string pref with locale-driven derivation per R3.
    let currency = resolve_string_pref(
        user.as_ref().and_then(|u| u.currency.clone()),
        org.as_ref().and_then(|o| o.currency.clone()),
        || locale_to_currency(&locale),
        &default.currency,
    );

    // -- theme: user-only. None at the user layer falls straight to
    // -- the system default; org layer carries no theme column.
    let theme = user.as_ref().and_then(|u| u.theme).unwrap_or(default.theme);

    ResolvedPreferences {
        timezone,
        locale,
        language,
        unit_system,
        temperature_unit,
        pressure_unit,
        speed_unit,
        length_unit,
        mass_unit,
        date_format,
        time_format,
        week_start,
        number_format,
        currency,
        theme,
    }
}

// ---------------------------------------------------------------------
// Per-field resolution helpers.
// ---------------------------------------------------------------------

fn first_string(user: Option<String>, org: Option<String>, default: &str) -> String {
    user.or(org).unwrap_or_else(|| default.to_owned())
}

fn first_copy<T: Copy>(user: Option<T>, org: Option<T>, default: T) -> T {
    user.or(org).unwrap_or(default)
}

/// Resolve a [`StringPref`] field (currency, timezone) per R3.
///
/// Precedence:
/// 1. Explicit value at any layer (user → org).
/// 2. Locale-derived default (caller supplies the derivation; receives
///    the already-resolved locale).
/// 3. Hardcoded system default.
///
/// An explicit `Auto` at any layer is **not** treated as a value — it
/// falls through to the derivation chain. This matches R3's "explicit
/// value at any layer" wording: `"auto"` is a placeholder meaning
/// *derive*, not a value.
fn resolve_string_pref(
    user: Option<StringPref>,
    org: Option<StringPref>,
    derive: impl FnOnce() -> Option<String>,
    default: &str,
) -> String {
    // Walk user → org for an explicit value.
    for layer in [user, org].into_iter().flatten() {
        if let StringPref::Explicit(v) = layer {
            return v;
        }
    }
    // Derive — the caller's closure captures whatever inputs the
    // derivation needs (the resolved locale, for currency; nothing,
    // for timezone where no derivation exists in v1).
    if let Some(v) = derive() {
        return v;
    }
    default.to_owned()
}

/// Resolve a per-unit field per R3's `"auto"` derivation order.
#[allow(clippy::too_many_arguments)]
fn resolve_unit(
    quantity: Quantity,
    user_field: Option<UnitPref>,
    org_field: Option<UnitPref>,
    user_system: Option<UnitSystem>,
    org_system: Option<UnitSystem>,
    default_system: UnitSystem,
    default_unit: Unit,
) -> Unit {
    // 1. Explicit value at any layer (user → org). `Auto` is not an
    //    explicit value per R3 — it's the placeholder meaning derive.
    for layer in [user_field, org_field].into_iter().flatten() {
        if let UnitPref::Explicit(u) = layer {
            return u;
        }
    }
    // 2. unit_system at the same/closer layer. "Same/closer" =
    //    walk user → org → default for the first non-null
    //    unit_system; map via the metric/imperial table.
    let system = user_system.or(org_system).unwrap_or(default_system);
    if let Some(u) = unit_for_system(quantity, system) {
        return u;
    }
    // 3. Locale-derived default (ICU) — wired in a later stage. For
    //    now fall through.
    // 4. Hardcoded system default.
    default_unit
}

/// `unit_system → unit` table per R3.
fn unit_for_system(quantity: Quantity, system: UnitSystem) -> Option<Unit> {
    Some(match (quantity, system) {
        (Quantity::Temperature, UnitSystem::Metric) => Unit::Celsius,
        (Quantity::Temperature, UnitSystem::Imperial) => Unit::Fahrenheit,
        (Quantity::Pressure, UnitSystem::Metric) => Unit::Kilopascal,
        (Quantity::Pressure, UnitSystem::Imperial) => Unit::Psi,
        (Quantity::Speed, UnitSystem::Metric) => Unit::KilometerPerHour,
        (Quantity::Speed, UnitSystem::Imperial) => Unit::MilePerHour,
        (Quantity::Length, UnitSystem::Metric) => Unit::Meter,
        (Quantity::Length, UnitSystem::Imperial) => Unit::Foot,
        (Quantity::Mass, UnitSystem::Metric) => Unit::Kilogram,
        (Quantity::Mass, UnitSystem::Imperial) => Unit::Pound,
        (Quantity::Volume, UnitSystem::Metric) => Unit::Liter,
        (Quantity::Volume, UnitSystem::Imperial) => Unit::GallonUs,
        (Quantity::Area, UnitSystem::Metric) => Unit::SquareMeter,
        (Quantity::Area, UnitSystem::Imperial) => Unit::SquareFoot,
        // Quantities with no metric/imperial split (duration, energy,
        // power, angle, frequency) fall through to the hardcoded
        // default in [`resolve_unit`].
        (Quantity::Duration, _)
        | (Quantity::Energy, _)
        | (Quantity::Power, _)
        | (Quantity::Angle, _)
        | (Quantity::Frequency, _) => return None,
    })
}

/// Resolve a display field that carries an `Auto` variant
/// (`DateFormat`, `TimeFormat`, `WeekStart`, `NumberFormat`).
///
/// First non-null, non-`Auto` value wins. Anything that resolves to
/// `Auto` falls through to the hardcoded default (ICU integration
/// lands in a later stage).
fn resolve_display<T: Copy + PartialEq>(user: Option<T>, org: Option<T>, auto: T, default: T) -> T {
    for layer in [user, org].into_iter().flatten() {
        if layer != auto {
            return layer;
        }
    }
    default
}

/// Derive a currency code from a BCP-47 locale tag via
/// `iso_currency`'s country → currency mapping.
///
/// Parses the region subtag (e.g. `en-AU` → `AU`) and looks up the
/// country's primary currency. Returns `None` if the locale has no
/// region subtag or the region is not a known ISO 3166-1 alpha-2
/// code. Per D-U0.3 this is the **only** place currency-data lives in
/// the workspace.
fn locale_to_currency(locale: &str) -> Option<String> {
    let region = bcp47_region(locale)?;
    let country: iso_currency::Country = region.parse().ok()?;
    Some(iso_currency::Currency::from(country).code().to_owned())
}

/// Extract the ISO 3166-1 alpha-2 region subtag from a BCP-47 tag.
/// Looks for a two-letter ASCII-alpha component after the language
/// subtag, normalised to uppercase. Returns `None` if absent.
fn bcp47_region(tag: &str) -> Option<String> {
    // BCP-47: language[-script][-region][-…]; the region subtag is
    // exactly two ASCII letters (a three-digit UN M49 numeric form
    // exists too but we don't ship a numeric → currency table).
    for part in tag.split('-').skip(1) {
        if part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            return Some(part.to_ascii_uppercase());
        }
    }
    None
}

#[cfg(test)]
mod tests;
