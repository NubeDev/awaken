//! The canonical `his` Arrow schema, shared by the SQLite hot tier, the
//! Parquet cold tier, and the union table provider.
//!
//! All three columns are `Utf8`: this mirrors the SQLite `his` table exactly
//! (`value` is the JSON-encoded `PointValue` text, cast to a number only at
//! rollup time), so a query reads identically across the tier boundary.

use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Build the `his` Arrow schema: `point_id`, `ts`, `value`, all nullable Utf8.
///
/// Nullability matches the SQLite provider: projections, filters, and outer
/// joins legitimately yield nulls regardless of the store's NOT NULL columns.
pub(crate) fn his_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("point_id", DataType::Utf8, true),
        Field::new("ts", DataType::Utf8, true),
        Field::new("value", DataType::Utf8, true),
    ]))
}
