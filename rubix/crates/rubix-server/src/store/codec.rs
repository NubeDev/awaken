//! JSON-column and RFC 3339 timestamp codecs shared by the row mappers.

use chrono::{DateTime, Utc};
use rubix_core::TagSet;

pub(crate) fn json_to<T: serde::de::DeserializeOwned>(s: &str) -> rusqlite::Result<T> {
    serde_json::from_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn json_of<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("domain types serialize")
}

pub(crate) fn ts_to(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub(crate) fn ts_of(t: &DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// In-Rust tag filtering over decoded rows; per-node datasets are small
/// enough that this beats maintaining JSON1 index SQL.
pub(crate) fn filter_tags<T>(
    items: Vec<T>,
    tags: &[String],
    get: impl Fn(&T) -> &TagSet,
) -> Vec<T> {
    if tags.is_empty() {
        return items;
    }
    items
        .into_iter()
        .filter(|item| get(item).has_all(tags.iter().map(String::as_str)))
        .collect()
}
