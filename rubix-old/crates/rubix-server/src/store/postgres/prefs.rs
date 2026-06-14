//! Preference rows, Postgres backend. Mirrors [`super::super::prefs`]. All
//! columns are nullable TEXT (the enum/`auto`/unit-code shapes the codecs round
//! trip), `updated_at` is BIGINT. Cloud-feature only.

use rubix_prefs::preferences::{
    DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
};
use rubix_prefs::resolver::{OrgPrefsRow, StringPref, UnitPref, UserPrefsRow};

use super::super::prefs::{
    enum_from_db, enum_to_db, now_epoch_ms, string_pref_to_db, unit_pref_from_db, unit_pref_to_db,
    ORG_COLS, USER_COLS,
};
use super::super::{Result, Store};

fn opt(row: &postgres::Row, col: &str) -> Option<String> {
    row.get::<_, Option<String>>(col)
}

fn opt_unit(row: &postgres::Row, col: &str) -> Result<Option<UnitPref>> {
    opt(row, col).map(|s| unit_pref_from_db(&s)).transpose()
}

fn opt_enum<T: serde::de::DeserializeOwned>(row: &postgres::Row, col: &str) -> Result<Option<T>> {
    opt(row, col).map(|s| enum_from_db::<T>(&s)).transpose()
}

fn row_user(row: &postgres::Row) -> Result<UserPrefsRow> {
    Ok(UserPrefsRow {
        timezone: opt(row, "timezone").map(|s| StringPref::parse(&s)),
        locale: opt(row, "locale"),
        language: opt(row, "language"),
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
        currency: opt(row, "currency").map(|s| StringPref::parse(&s)),
        theme: opt_enum::<Theme>(row, "theme")?,
    })
}

fn row_org(row: &postgres::Row) -> Result<OrgPrefsRow> {
    Ok(OrgPrefsRow {
        timezone: opt(row, "timezone").map(|s| StringPref::parse(&s)),
        locale: opt(row, "locale"),
        language: opt(row, "language"),
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
        currency: opt(row, "currency").map(|s| StringPref::parse(&s)),
    })
}

pub(crate) fn get_user_prefs(
    store: &Store,
    user_id: &str,
    org: &str,
) -> Result<Option<UserPrefsRow>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {USER_COLS} FROM prefs_user WHERE user_id = $1 AND org = $2");
    match client.query_opt(sql.as_str(), &[&user_id, &org])? {
        Some(row) => Ok(Some(row_user(&row)?)),
        None => Ok(None),
    }
}

pub(crate) fn get_org_prefs(store: &Store, org: &str) -> Result<Option<OrgPrefsRow>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {ORG_COLS} FROM prefs_org WHERE org = $1");
    match client.query_opt(sql.as_str(), &[&org])? {
        Some(row) => Ok(Some(row_org(&row)?)),
        None => Ok(None),
    }
}

pub(crate) fn upsert_user_prefs(
    store: &Store,
    user_id: &str,
    org: &str,
    patch: &UserPrefsRow,
) -> Result<()> {
    let mut client = store.postgres_conn()?;
    client.execute(
        "INSERT INTO prefs_user ( \
            user_id, org, timezone, locale, language, unit_system, \
            temperature_unit, pressure_unit, speed_unit, length_unit, mass_unit, \
            date_format, time_format, week_start, number_format, currency, theme, \
            updated_at \
         ) VALUES ( \
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18 \
         ) \
         ON CONFLICT (user_id, org) DO UPDATE SET \
            timezone=EXCLUDED.timezone, locale=EXCLUDED.locale, language=EXCLUDED.language, \
            unit_system=EXCLUDED.unit_system, temperature_unit=EXCLUDED.temperature_unit, \
            pressure_unit=EXCLUDED.pressure_unit, speed_unit=EXCLUDED.speed_unit, \
            length_unit=EXCLUDED.length_unit, mass_unit=EXCLUDED.mass_unit, \
            date_format=EXCLUDED.date_format, time_format=EXCLUDED.time_format, \
            week_start=EXCLUDED.week_start, number_format=EXCLUDED.number_format, \
            currency=EXCLUDED.currency, theme=EXCLUDED.theme, updated_at=EXCLUDED.updated_at",
        &[
            &user_id,
            &org,
            &patch.timezone.as_ref().map(string_pref_to_db),
            &patch.locale,
            &patch.language,
            &patch.unit_system.map(enum_to_db),
            &patch.temperature_unit.as_ref().map(unit_pref_to_db),
            &patch.pressure_unit.as_ref().map(unit_pref_to_db),
            &patch.speed_unit.as_ref().map(unit_pref_to_db),
            &patch.length_unit.as_ref().map(unit_pref_to_db),
            &patch.mass_unit.as_ref().map(unit_pref_to_db),
            &patch.date_format.map(enum_to_db),
            &patch.time_format.map(enum_to_db),
            &patch.week_start.map(enum_to_db),
            &patch.number_format.map(enum_to_db),
            &patch.currency.as_ref().map(string_pref_to_db),
            &patch.theme.map(enum_to_db),
            &now_epoch_ms(),
        ],
    )?;
    Ok(())
}

pub(crate) fn upsert_org_prefs(store: &Store, org: &str, patch: &OrgPrefsRow) -> Result<()> {
    let mut client = store.postgres_conn()?;
    client.execute(
        "INSERT INTO prefs_org ( \
            org, timezone, locale, language, unit_system, temperature_unit, \
            pressure_unit, speed_unit, length_unit, mass_unit, date_format, \
            time_format, week_start, number_format, currency, updated_at \
         ) VALUES ( \
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16 \
         ) \
         ON CONFLICT (org) DO UPDATE SET \
            timezone=EXCLUDED.timezone, locale=EXCLUDED.locale, language=EXCLUDED.language, \
            unit_system=EXCLUDED.unit_system, temperature_unit=EXCLUDED.temperature_unit, \
            pressure_unit=EXCLUDED.pressure_unit, speed_unit=EXCLUDED.speed_unit, \
            length_unit=EXCLUDED.length_unit, mass_unit=EXCLUDED.mass_unit, \
            date_format=EXCLUDED.date_format, time_format=EXCLUDED.time_format, \
            week_start=EXCLUDED.week_start, number_format=EXCLUDED.number_format, \
            currency=EXCLUDED.currency, updated_at=EXCLUDED.updated_at",
        &[
            &org,
            &patch.timezone.as_ref().map(string_pref_to_db),
            &patch.locale,
            &patch.language,
            &patch.unit_system.map(enum_to_db),
            &patch.temperature_unit.as_ref().map(unit_pref_to_db),
            &patch.pressure_unit.as_ref().map(unit_pref_to_db),
            &patch.speed_unit.as_ref().map(unit_pref_to_db),
            &patch.length_unit.as_ref().map(unit_pref_to_db),
            &patch.mass_unit.as_ref().map(unit_pref_to_db),
            &patch.date_format.map(enum_to_db),
            &patch.time_format.map(enum_to_db),
            &patch.week_start.map(enum_to_db),
            &patch.number_format.map(enum_to_db),
            &patch.currency.as_ref().map(string_pref_to_db),
            &now_epoch_ms(),
        ],
    )?;
    Ok(())
}
