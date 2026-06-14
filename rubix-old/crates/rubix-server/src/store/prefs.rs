//! Units & datetime preference rows (WS-11): the org and user layers, both
//! all-nullable (NULL = inherit). The store is a faithful row mirror — NULL on
//! disk comes back as `None`, `"auto"` stays `"auto"`, a concrete code stays
//! itself. The three-layer collapse (user → org → system default) and the
//! `"auto"` derivation are [`rubix_prefs::resolver`]'s job, not the store's.
//!
//! Vendored shapes (`UserPrefsRow` / `OrgPrefsRow` / `UnitPref` / `StringPref`)
//! live in `rubix-prefs`; this module persists them against rubix's synchronous
//! `Store`. Backend dispatch; SQLite body inline, Postgres body in
//! [`super::postgres::prefs`]. See WS-11 and `store/users.rs` for the pattern.

use std::str::FromStr;

use rusqlite::{params, OptionalExtension, Row};
use serde::{de::DeserializeOwned, Serialize};

use rubix_prefs::preferences::{
    DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
};
use rubix_prefs::resolver::{OrgPrefsRow, StringPref, UnitPref, UserPrefsRow};
use rubix_prefs::units::Unit;

use super::backend::Backend;
use super::{Result, Store, StoreError};

// ---------------------------------------------------------------------
// DB <-> Row column codecs (backend-agnostic; operate on TEXT).
//
// Mirror the column shapes the schema declares: enum wire tokens, the
// "auto" sentinel, or a concrete unit code. NULL stays NULL, "auto" stays
// "auto", an explicit value stays the same string — the round-trip the
// tests check.
// ---------------------------------------------------------------------

fn decode_err(msg: String) -> StoreError {
    StoreError::Db(anyhow::anyhow!(msg))
}

pub(super) fn enum_to_db<T: Serialize>(v: T) -> String {
    match serde_json::to_value(v).expect("prefs enum serializes to JSON") {
        serde_json::Value::String(s) => s,
        other => panic!("expected JSON string for prefs enum, got {other:?}"),
    }
}

pub(super) fn enum_from_db<T: DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value::<T>(serde_json::Value::String(s.to_owned()))
        .map_err(|e| decode_err(format!("invalid prefs enum {s:?}: {e}")))
}

pub(super) fn unit_pref_to_db(v: &UnitPref) -> String {
    match v {
        UnitPref::Auto => "auto".to_owned(),
        UnitPref::Explicit(u) => u.as_str().to_owned(),
    }
}

pub(super) fn unit_pref_from_db(s: &str) -> Result<UnitPref> {
    if s == "auto" {
        Ok(UnitPref::Auto)
    } else {
        Unit::from_str(s)
            .map(UnitPref::Explicit)
            .map_err(|e| decode_err(format!("invalid unit code {s:?}: {e}")))
    }
}

pub(super) fn string_pref_to_db(v: &StringPref) -> String {
    match v {
        StringPref::Auto => "auto".to_owned(),
        StringPref::Explicit(s) => s.clone(),
    }
}

pub(super) fn now_epoch_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------
// SQLite row decoders.
// ---------------------------------------------------------------------

fn opt_text(row: &Row<'_>, col: &str) -> rusqlite::Result<Option<String>> {
    row.get::<_, Option<String>>(col)
}

fn row_user(row: &Row<'_>) -> Result<UserPrefsRow> {
    Ok(UserPrefsRow {
        timezone: opt_text(row, "timezone")?.map(|s| StringPref::parse(&s)),
        locale: opt_text(row, "locale")?,
        language: opt_text(row, "language")?,
        unit_system: opt_enum::<UnitSystem>(row, "unit_system")?,
        temperature_unit: opt_unit(row, "temperature_unit")?,
        pressure_unit: opt_unit(row, "pressure_unit")?,
        speed_unit: opt_unit(row, "speed_unit")?,
        length_unit: opt_unit(row, "length_unit")?,
        mass_unit: opt_unit(row, "mass_unit")?,
        date_format: opt_enum::<DateFormat>(row, "date_format")?,
        time_format: opt_enum::<TimeFormat>(row, "time_format")?,
        week_start: opt_enum::<WeekStart>(row, "week_start")?,
        number_format: opt_enum::<NumberFormat>(row, "number_format")?,
        currency: opt_text(row, "currency")?.map(|s| StringPref::parse(&s)),
        theme: opt_enum::<Theme>(row, "theme")?,
    })
}

fn row_org(row: &Row<'_>) -> Result<OrgPrefsRow> {
    Ok(OrgPrefsRow {
        timezone: opt_text(row, "timezone")?.map(|s| StringPref::parse(&s)),
        locale: opt_text(row, "locale")?,
        language: opt_text(row, "language")?,
        unit_system: opt_enum::<UnitSystem>(row, "unit_system")?,
        temperature_unit: opt_unit(row, "temperature_unit")?,
        pressure_unit: opt_unit(row, "pressure_unit")?,
        speed_unit: opt_unit(row, "speed_unit")?,
        length_unit: opt_unit(row, "length_unit")?,
        mass_unit: opt_unit(row, "mass_unit")?,
        date_format: opt_enum::<DateFormat>(row, "date_format")?,
        time_format: opt_enum::<TimeFormat>(row, "time_format")?,
        week_start: opt_enum::<WeekStart>(row, "week_start")?,
        number_format: opt_enum::<NumberFormat>(row, "number_format")?,
        currency: opt_text(row, "currency")?.map(|s| StringPref::parse(&s)),
    })
}

fn opt_unit(row: &Row<'_>, col: &str) -> Result<Option<UnitPref>> {
    opt_text(row, col)?.map(|s| unit_pref_from_db(&s)).transpose()
}

fn opt_enum<T: DeserializeOwned>(row: &Row<'_>, col: &str) -> Result<Option<T>> {
    opt_text(row, col)?.map(|s| enum_from_db::<T>(&s)).transpose()
}

/// The user-layer columns in canonical order — shared with the Postgres path so
/// the two decoders read the same shape.
pub(super) const USER_COLS: &str = "timezone, locale, language, unit_system, \
    temperature_unit, pressure_unit, speed_unit, length_unit, mass_unit, \
    date_format, time_format, week_start, number_format, currency, theme";

/// The org-layer columns (no `theme`).
pub(super) const ORG_COLS: &str = "timezone, locale, language, unit_system, \
    temperature_unit, pressure_unit, speed_unit, length_unit, mass_unit, \
    date_format, time_format, week_start, number_format, currency";

impl Store {
    /// Fetch the user-layer prefs row for `(user_id, org)`. `None` when the user
    /// has never written prefs in that org.
    pub fn get_user_prefs(&self, user_id: &str, org: &str) -> Result<Option<UserPrefsRow>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                conn.query_row(
                    &format!(
                        "SELECT {USER_COLS} FROM prefs_user WHERE user_id = ?1 AND org = ?2"
                    ),
                    params![user_id, org],
                    |r| Ok(row_user(r)),
                )
                .optional()?
                .transpose()
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::prefs::get_user_prefs(self, user_id, org),
        }
    }

    /// Fetch the org-layer prefs row for `org`. `None` when unset.
    pub fn get_org_prefs(&self, org: &str) -> Result<Option<OrgPrefsRow>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                conn.query_row(
                    &format!("SELECT {ORG_COLS} FROM prefs_org WHERE org = ?1"),
                    params![org],
                    |r| Ok(row_org(r)),
                )
                .optional()?
                .transpose()
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::prefs::get_org_prefs(self, org),
        }
    }

    /// Insert or update the user-layer row. Every column is written: `None`
    /// becomes SQL NULL ("inherit"). `updated_at` is stamped server-side.
    pub fn upsert_user_prefs(&self, user_id: &str, org: &str, patch: &UserPrefsRow) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "INSERT INTO prefs_user ( \
                        user_id, org, timezone, locale, language, unit_system, \
                        temperature_unit, pressure_unit, speed_unit, length_unit, \
                        mass_unit, date_format, time_format, week_start, \
                        number_format, currency, theme, updated_at \
                     ) VALUES ( \
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, \
                        ?14, ?15, ?16, ?17, ?18 \
                     ) \
                     ON CONFLICT(user_id, org) DO UPDATE SET \
                        timezone=excluded.timezone, locale=excluded.locale, \
                        language=excluded.language, unit_system=excluded.unit_system, \
                        temperature_unit=excluded.temperature_unit, \
                        pressure_unit=excluded.pressure_unit, speed_unit=excluded.speed_unit, \
                        length_unit=excluded.length_unit, mass_unit=excluded.mass_unit, \
                        date_format=excluded.date_format, time_format=excluded.time_format, \
                        week_start=excluded.week_start, number_format=excluded.number_format, \
                        currency=excluded.currency, theme=excluded.theme, \
                        updated_at=excluded.updated_at",
                    params![
                        user_id,
                        org,
                        patch.timezone.as_ref().map(string_pref_to_db),
                        patch.locale,
                        patch.language,
                        patch.unit_system.map(enum_to_db),
                        patch.temperature_unit.as_ref().map(unit_pref_to_db),
                        patch.pressure_unit.as_ref().map(unit_pref_to_db),
                        patch.speed_unit.as_ref().map(unit_pref_to_db),
                        patch.length_unit.as_ref().map(unit_pref_to_db),
                        patch.mass_unit.as_ref().map(unit_pref_to_db),
                        patch.date_format.map(enum_to_db),
                        patch.time_format.map(enum_to_db),
                        patch.week_start.map(enum_to_db),
                        patch.number_format.map(enum_to_db),
                        patch.currency.as_ref().map(string_pref_to_db),
                        patch.theme.map(enum_to_db),
                        now_epoch_ms(),
                    ],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::prefs::upsert_user_prefs(self, user_id, org, patch)
            }
        }
    }

    /// Insert or update the org-layer row. Same semantics as the user upsert.
    pub fn upsert_org_prefs(&self, org: &str, patch: &OrgPrefsRow) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "INSERT INTO prefs_org ( \
                        org, timezone, locale, language, unit_system, \
                        temperature_unit, pressure_unit, speed_unit, length_unit, \
                        mass_unit, date_format, time_format, week_start, \
                        number_format, currency, updated_at \
                     ) VALUES ( \
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, \
                        ?14, ?15, ?16 \
                     ) \
                     ON CONFLICT(org) DO UPDATE SET \
                        timezone=excluded.timezone, locale=excluded.locale, \
                        language=excluded.language, unit_system=excluded.unit_system, \
                        temperature_unit=excluded.temperature_unit, \
                        pressure_unit=excluded.pressure_unit, speed_unit=excluded.speed_unit, \
                        length_unit=excluded.length_unit, mass_unit=excluded.mass_unit, \
                        date_format=excluded.date_format, time_format=excluded.time_format, \
                        week_start=excluded.week_start, number_format=excluded.number_format, \
                        currency=excluded.currency, updated_at=excluded.updated_at",
                    params![
                        org,
                        patch.timezone.as_ref().map(string_pref_to_db),
                        patch.locale,
                        patch.language,
                        patch.unit_system.map(enum_to_db),
                        patch.temperature_unit.as_ref().map(unit_pref_to_db),
                        patch.pressure_unit.as_ref().map(unit_pref_to_db),
                        patch.speed_unit.as_ref().map(unit_pref_to_db),
                        patch.length_unit.as_ref().map(unit_pref_to_db),
                        patch.mass_unit.as_ref().map(unit_pref_to_db),
                        patch.date_format.map(enum_to_db),
                        patch.time_format.map(enum_to_db),
                        patch.week_start.map(enum_to_db),
                        patch.number_format.map(enum_to_db),
                        patch.currency.as_ref().map(string_pref_to_db),
                        now_epoch_ms(),
                    ],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::prefs::upsert_org_prefs(self, org, patch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubix_prefs::units::Unit;

    fn store() -> Store {
        // A fresh in-memory-style temp file store; the base schema + migrations
        // create the prefs tables.
        let dir = tempfile::tempdir().unwrap();
        Store::open(&dir.path().join("t.db")).unwrap()
    }

    #[test]
    fn user_prefs_round_trip_null_auto_and_explicit() {
        let s = store();
        let patch = UserPrefsRow {
            timezone: Some(StringPref::Explicit("Europe/Paris".into())),
            locale: Some("fr-FR".into()),
            language: None, // NULL = inherit
            unit_system: Some(UnitSystem::Imperial),
            temperature_unit: Some(UnitPref::Explicit(Unit::Fahrenheit)),
            pressure_unit: Some(UnitPref::Auto), // "auto" sentinel
            speed_unit: None,
            length_unit: None,
            mass_unit: None,
            date_format: Some(DateFormat::IsoYMD),
            time_format: None,
            week_start: None,
            number_format: None,
            currency: Some(StringPref::Auto),
            theme: Some(Theme::Dark),
        };
        s.upsert_user_prefs("user-1", "nube", &patch).unwrap();
        let got = s.get_user_prefs("user-1", "nube").unwrap().unwrap();
        assert_eq!(got, patch);
        // A different org is isolated.
        assert!(s.get_user_prefs("user-1", "acme").unwrap().is_none());
    }

    #[test]
    fn user_upsert_overwrites() {
        let s = store();
        let mut patch = UserPrefsRow::default();
        patch.theme = Some(Theme::Light);
        s.upsert_user_prefs("u", "nube", &patch).unwrap();
        patch.theme = Some(Theme::Dark);
        s.upsert_user_prefs("u", "nube", &patch).unwrap();
        assert_eq!(s.get_user_prefs("u", "nube").unwrap().unwrap().theme, Some(Theme::Dark));
    }

    #[test]
    fn org_prefs_round_trip() {
        let s = store();
        let patch = OrgPrefsRow {
            timezone: None,
            locale: Some("en-AU".into()),
            language: None,
            unit_system: Some(UnitSystem::Metric),
            temperature_unit: None,
            pressure_unit: None,
            speed_unit: Some(UnitPref::Explicit(Unit::Knot)),
            length_unit: None,
            mass_unit: None,
            date_format: None,
            time_format: Some(TimeFormat::H24),
            week_start: None,
            number_format: None,
            currency: Some(StringPref::Explicit("AUD".into())),
        };
        s.upsert_org_prefs("nube", &patch).unwrap();
        assert_eq!(s.get_org_prefs("nube").unwrap().unwrap(), patch);
        assert!(s.get_org_prefs("acme").unwrap().is_none());
    }
}
