//! Discover the bindable facets of a canonical table, scoped.
//!
//! Authoring a rule binding means naming three things the author cannot see: the
//! numeric series to roll up (`field`), the key to narrow it by (`filter_field`),
//! and the value to match (`filter_value`). This module introspects what the
//! table *actually holds* — through the principal's scoped session, so the facets
//! are exactly the rows SurrealDB permissions admit (contract #1) — and reports
//! them so the studio can offer them instead of making the author type blind.
//!
//! The discovery mirrors how a binding *resolves* ([`super::series`]), so what is
//! offered is exactly what a binding can bind: for [`CanonicalTable::Readings`]
//! the typed plane, the field is the top-level `value` and the only narrowing key
//! is the top-level `series` link; for every other (generic record) table the
//! fields are the numeric `content.<key>`s and the narrowing keys are the string
//! `content.<key>`s, each with its distinct values. Values per key are bounded by
//! [`VALUE_CAP`] so a wide series space (thousands of points) stays a usable
//! picker rather than an unbounded payload — a capped facet says so explicitly
//! ([`FilterFacet::truncated`]) rather than silently pretending it is complete.

use std::collections::{BTreeMap, BTreeSet};

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};
use crate::provider::CanonicalTable;

use super::series::record_key;

/// The most distinct values a single filter key reports.
///
/// A point space can run to thousands of series; an author picks from a list, so
/// past a couple hundred the value is a search box, not a longer dropdown. The cap
/// bounds the payload and the facet flags when it bit (`truncated`).
const VALUE_CAP: usize = 200;

/// One narrowing key a binding can scope its series by, with the distinct values
/// observed for it.
///
/// For a reading binding this is `series` and the values are point ids; for a
/// record binding it is a `content` key (e.g. `measure`) and the values its
/// categories (`temp`, `co2`, …). The pair `(key, value)` is exactly a
/// [`SeriesFilter`](super::SeriesFilter) the author can apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterFacet {
    /// The key a binding's `filter_field` would take.
    pub key: String,
    /// The distinct values observed for `key`, sorted, capped at [`VALUE_CAP`].
    pub values: Vec<String>,
    /// Whether more distinct values exist than the cap returned — the list is a
    /// sample, not the whole space.
    pub truncated: bool,
}

/// The bindable facets of one canonical table: the numeric series it offers and
/// the keys those series can be narrowed by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableFacets {
    /// The numeric fields a binding's `field` can take, sorted. For a reading
    /// table this is the single typed `value`; for a record table the numeric
    /// `content.<key>`s seen.
    pub fields: Vec<String>,
    /// The keys a binding's `filter_field` can take, each with its distinct
    /// values, sorted by key.
    pub filters: Vec<FilterFacet>,
}

/// Discover what `table` offers a binding, scoped to the principal of `session`.
///
/// Scans the table on the scoped session (the facets are bounded by the rows the
/// principal may read, contract #1) and folds each row into the fields and filter
/// values it contributes. The scan is read-only and side-effect-free, like the
/// dry-run it backs.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the scoped read fails.
pub async fn discover_facets(session: &Surreal<Db>, table: CanonicalTable) -> Result<TableFacets> {
    let surreal_table = table.surreal_table();
    let mut response = session
        .query(format!("SELECT * FROM {surreal_table}"))
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let rows: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;

    // Discovery must match resolution: a reading binds the typed top-level
    // `value`/`series`, every other table binds free-form `content.<key>`.
    if matches!(table, CanonicalTable::Readings) {
        Ok(reading_facets(&rows))
    } else {
        Ok(record_facets(&rows))
    }
}

/// Facets of the typed reading plane: the field is always `value`; the one
/// narrowing key is `series`, whose values are the distinct point ids (normalised
/// from the record link to the bare key, as a binding's filter matches them).
fn reading_facets(rows: &[serde_json::Value]) -> TableFacets {
    let mut series: BTreeSet<String> = BTreeSet::new();
    let mut truncated = false;
    for row in rows {
        let Some(value) = row.get("series") else {
            continue;
        };
        let key = record_key(value);
        if series.len() < VALUE_CAP || series.contains(&key) {
            series.insert(key);
        } else {
            truncated = true;
        }
    }

    // No readings → no series to narrow by, but the field is still bindable.
    let filters = if series.is_empty() {
        Vec::new()
    } else {
        vec![FilterFacet {
            key: "series".to_owned(),
            values: series.into_iter().collect(),
            truncated,
        }]
    };
    TableFacets {
        fields: vec!["value".to_owned()],
        filters,
    }
}

/// Facets of a generic record table: numeric `content.<key>`s are bindable
/// fields; string `content.<key>`s are narrowing keys, each carrying its distinct
/// values. A key seen as both number and string appears in both — the rule may
/// roll it up *and* another binding narrow on it.
fn record_facets(rows: &[serde_json::Value]) -> TableFacets {
    let mut fields: BTreeSet<String> = BTreeSet::new();
    let mut filters: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut truncated: BTreeSet<String> = BTreeSet::new();

    for row in rows {
        let Some(content) = row.get("content").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (key, value) in content {
            if value.is_number() {
                fields.insert(key.clone());
            } else if let Some(text) = value.as_str() {
                let seen = filters.entry(key.clone()).or_default();
                if seen.len() < VALUE_CAP || seen.contains(text) {
                    seen.insert(text.to_owned());
                } else {
                    truncated.insert(key.clone());
                }
            }
        }
    }

    TableFacets {
        fields: fields.into_iter().collect(),
        filters: filters
            .into_iter()
            .map(|(key, values)| FilterFacet {
                truncated: truncated.contains(&key),
                key,
                values: values.into_iter().collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{record_facets, reading_facets};

    #[test]
    fn reading_facets_offer_value_and_distinct_series() {
        let rows = vec![
            serde_json::json!({ "series": "reading:hq--ahu-1--temp", "value": 21.0 }),
            serde_json::json!({ "series": "reading:hq--ahu-2--temp", "value": 22.0 }),
            serde_json::json!({ "series": "reading:hq--ahu-1--temp", "value": 23.0 }),
        ];
        let facets = reading_facets(&rows);
        assert_eq!(facets.fields, ["value"]);
        assert_eq!(facets.filters.len(), 1);
        let series = &facets.filters[0];
        assert_eq!(series.key, "series");
        // Distinct, normalised to the bare key, sorted; the repeat collapses.
        assert_eq!(series.values, ["hq--ahu-1--temp", "hq--ahu-2--temp"]);
        assert!(!series.truncated);
    }

    #[test]
    fn no_readings_yields_a_bindable_field_but_no_series_facet() {
        let facets = reading_facets(&[]);
        assert_eq!(facets.fields, ["value"]);
        assert!(facets.filters.is_empty());
    }

    #[test]
    fn record_facets_split_numeric_fields_from_string_filters() {
        let rows = vec![
            serde_json::json!({ "content": { "value": 21.5, "measure": "temp", "site": "hq" } }),
            serde_json::json!({ "content": { "value": 410.0, "measure": "co2", "site": "hq" } }),
        ];
        let facets = record_facets(&rows);
        assert_eq!(facets.fields, ["value"]);
        let keys: Vec<&str> = facets.filters.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["measure", "site"]);
        let measure = facets.filters.iter().find(|f| f.key == "measure").unwrap();
        assert_eq!(measure.values, ["co2", "temp"]);
        let site = facets.filters.iter().find(|f| f.key == "site").unwrap();
        assert_eq!(site.values, ["hq"]);
    }

    #[test]
    fn a_row_without_content_is_skipped() {
        let rows = vec![serde_json::json!({ "id": "x" })];
        assert_eq!(record_facets(&rows), super::TableFacets::default());
    }
}
