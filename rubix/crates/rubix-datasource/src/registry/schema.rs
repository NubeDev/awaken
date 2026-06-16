//! Enumerate the tables + columns the principal can read (§4b).
//!
//! `GET /query/schema` (`rubix/docs/design/DASHBOARDS-SCOPE.md` §4b) backs query
//! autocomplete and stops charts guessing the JSON shape. It is the read-only,
//! shape-only twin of [`span`](super::span): the **same** `external-query`
//! capability check and the **same** per-principal spanning context (so it reuses
//! the scoped scan cached in §4a), but instead of running a statement it walks the
//! built DataFusion catalog and reports each table's columns.
//!
//! Row-perm awareness: the native canonical tables are scanned through the
//! principal's scoped session, so a principal only sees a table at all if its
//! scoped scan resolved — the catalog reflects what they can read, never an
//! unscoped base table (contract #1). External tables are only present when the
//! caller holds `external-query` and the connector is registered. Columns are
//! reported with the **same coarse type tags** the query result path uses
//! (`columns_of`), so autocomplete and FieldConfig matching see one type model.

use datafusion::arrow::datatypes::DataType;
use rubix_gate::ScopedSession;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::ContextCache;

use super::span::{authorize_query, build_spanning_context};
use super::store::Registry;
use crate::error::{DatasourceError, Result};

/// One readable table and its columns, as the principal would address it in SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSchema {
    /// The schema (namespace) the table lives under: the default catalog schema
    /// for native canonical tables, or a datasource id for external tables.
    pub schema: String,
    /// The bare table name (e.g. `record`, or `sensor_readings`).
    pub table: String,
    /// The table's columns in declaration order.
    pub columns: Vec<ColumnSchema>,
}

/// One column's name and coarse type tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSchema {
    /// The column name.
    pub name: String,
    /// A coarse type tag (`number`/`string`/`boolean`/`timestamp`/`other`) — the
    /// same vocabulary `rubix-server`'s `columns_of` reports for result columns.
    pub kind: &'static str,
}

/// List every table + columns the principal of `session` can read.
///
/// Gated on the same `external-query` capability as [`span`](super::span) and
/// built on the same per-principal spanning context, so the cached scoped scan
/// (§4a) is reused. Walks the default catalog: every schema, every table, every
/// column. Tables are returned in catalog order (schema, then table name).
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the query
///   capability check.
/// - [`DatasourceError::DataFusion`] if building the context or reading a provider
///   schema fails.
pub async fn schema_of(
    registry: &Registry,
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
    cache: &ContextCache,
) -> Result<Vec<TableSchema>> {
    authorize_query(grant_reader, session).await?;
    let ctx = build_spanning_context(registry, session, cache).await?;

    let default_catalog = ctx.copied_config().options().catalog.default_catalog.clone();
    let catalog = ctx.catalog(&default_catalog).ok_or_else(|| {
        DatasourceError::Query(format!("missing default catalog `{default_catalog}`"))
    })?;

    let mut tables = Vec::new();
    let mut schema_names = catalog.schema_names();
    schema_names.sort();
    for schema_name in schema_names {
        let Some(schema) = catalog.schema(&schema_name) else {
            continue;
        };
        let mut table_names = schema.table_names();
        table_names.sort();
        for table_name in table_names {
            let Some(provider) = schema
                .table(&table_name)
                .await
                .map_err(DatasourceError::DataFusion)?
            else {
                continue;
            };
            let columns = provider
                .schema()
                .fields()
                .iter()
                .map(|field| ColumnSchema {
                    name: field.name().clone(),
                    kind: type_tag(field.data_type()),
                })
                .collect();
            tables.push(TableSchema {
                schema: schema_name.clone(),
                table: table_name,
                columns,
            });
        }
    }
    Ok(tables)
}

/// A coarse, client-friendly type tag for an Arrow [`DataType`].
///
/// Kept identical to `rubix-server`'s result-column `type_tag` so autocomplete and
/// result columns speak one type vocabulary (§4b: "the same coarse type tags as
/// columns_of").
fn type_tag(data_type: &DataType) -> &'static str {
    use DataType::{
        Boolean, Date32, Date64, Float16, Float32, Float64, Int8, Int16, Int32, Int64, LargeUtf8,
        Timestamp, UInt8, UInt16, UInt32, UInt64, Utf8,
    };
    match data_type {
        Int8 | Int16 | Int32 | Int64 | UInt8 | UInt16 | UInt32 | UInt64 | Float16 | Float32
        | Float64 => "number",
        Utf8 | LargeUtf8 => "string",
        Boolean => "boolean",
        Timestamp(_, _) | Date32 | Date64 => "timestamp",
        _ => "other",
    }
}
