//! The canonical tables and their Arrow schemas.
//!
//! DataFusion needs a typed schema to plan against; SurrealDB records are
//! schemaless documents (`rubix/docs/SCOPE.md`, principle 4). This file maps the
//! few canonical tables the query surface exposes — records, tags, audit,
//! insights, trace summaries — to a fixed Arrow schema with the structural columns every record
//! carries (`id`, `namespace`, `created`, `updated`) plus a `content` column
//! holding the free-form document JSON as a UTF-8 string. The free-form shape is
//! never flattened into columns: queries that need a field reach into `content`
//! with DataFusion's JSON functions, so the schemaless contract is preserved
//! above the engine.

use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};

/// A canonical table the read-only query surface exposes.
///
/// These are the tables `rubix-store` declares and later workstreams write to;
/// the query surface scans them read-only. `Insights` is the rule-decision table
/// WS-11 will write — it is declared here so the schema is stable before that
/// writer lands, and a scan of an empty table returns no rows (not an error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalTable {
    /// Generic document records (`rubix-core` `record`).
    Records,
    /// The tag vertices of the `record→tagged→tag` graph (`rubix-core` `tag`).
    Tags,
    /// The immutable audit log (`rubix-gate` `audit`).
    Audit,
    /// Rule decisions / insights (written by the Rhai runtime, WS-11).
    Insights,
    /// Per-correlation-id trace rollups (`rubix-trace` `trace_summary`, §5b).
    ///
    /// A Tier-B derived rollup, one row per trace, folded by `upsert_summary`
    /// (`rubix/docs/design/LAMINAR-BORROW.md` §5b/§7). Its fields (`status`,
    /// `num_spans`, `total_tokens`, …) live in the `content` JSON and are reached
    /// with `json_get`, the same as the other canonical tables.
    TraceSummary,
    /// The time-series data plane (`rubix-core` `reading`,
    /// `rubix/docs/design/READINGS-TIMESERIES.md`).
    ///
    /// Unlike the other tables, a reading is not a free-form document: its hot
    /// columns are typed. So this table carries its **own** 6-column Arrow
    /// schema (`id/namespace/series/at/value/created`) rather than the shared
    /// structural-plus-`content` shape — the query layer buckets on the
    /// measurement instant `at` and the numeric `value` directly, closing the
    /// trend-collapse bug that buckets on write time.
    Readings,
}

impl CanonicalTable {
    /// Every canonical table the surface exposes, in declaration order.
    pub const ALL: [CanonicalTable; 6] = [
        CanonicalTable::Records,
        CanonicalTable::Tags,
        CanonicalTable::Audit,
        CanonicalTable::Insights,
        CanonicalTable::TraceSummary,
        CanonicalTable::Readings,
    ];

    /// The SurrealDB table name scanned for this canonical table.
    #[must_use]
    pub fn surreal_table(self) -> &'static str {
        match self {
            CanonicalTable::Records => "record",
            CanonicalTable::Tags => "tag",
            CanonicalTable::Audit => "audit",
            CanonicalTable::Insights => "insight",
            CanonicalTable::TraceSummary => "trace_summary",
            CanonicalTable::Readings => "reading",
        }
    }

    /// The name this table is registered under in the DataFusion catalog.
    ///
    /// Kept equal to the SurrealDB table name so a principal writes the same name
    /// in SQL as the underlying store table — no aliasing layer to reason about.
    #[must_use]
    pub fn register_name(self) -> &'static str {
        self.surreal_table()
    }

    /// Resolve a SQL table name to a canonical table, or `None` if unknown.
    #[must_use]
    pub fn parse(name: &str) -> Option<CanonicalTable> {
        CanonicalTable::ALL
            .into_iter()
            .find(|table| table.register_name() == name)
    }

    /// The Arrow schema this table is scanned into.
    ///
    /// Most canonical tables share the structural columns; `content` carries the
    /// free-form document as JSON text so no domain shape is baked into the
    /// schema (`rubix/docs/SCOPE.md`, principle 4). [`CanonicalTable::Readings`]
    /// is the exception: a reading's hot columns are typed, so it gets its own
    /// 6-column schema with the measurement instant `at` and numeric `value`
    /// surfaced directly (`rubix/docs/design/READINGS-TIMESERIES.md`, "Read
    /// path").
    #[must_use]
    pub fn arrow_schema(self) -> Arc<Schema> {
        match self {
            CanonicalTable::Readings => Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("namespace", DataType::Utf8, true),
                Field::new("series", DataType::Utf8, true),
                Field::new(
                    "at",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new("value", DataType::Float64, true),
                Field::new(
                    "created",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
            ])),
            _ => Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("namespace", DataType::Utf8, true),
                Field::new(
                    "created",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new(
                    "updated",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Field::new("content", DataType::Utf8, true),
            ])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CanonicalTable;

    #[test]
    fn every_table_round_trips_through_its_register_name() {
        for table in CanonicalTable::ALL {
            assert_eq!(CanonicalTable::parse(table.register_name()), Some(table));
        }
    }

    #[test]
    fn an_unknown_table_resolves_to_none() {
        assert_eq!(CanonicalTable::parse("not_a_table"), None);
    }

    #[test]
    fn schema_carries_the_structural_columns_and_content() {
        let schema = CanonicalTable::Records.arrow_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(names, ["id", "namespace", "created", "updated", "content"]);
    }

    #[test]
    fn readings_carries_its_own_typed_time_series_columns() {
        let schema = CanonicalTable::Readings.arrow_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(names, ["id", "namespace", "series", "at", "value", "created"]);
    }
}
