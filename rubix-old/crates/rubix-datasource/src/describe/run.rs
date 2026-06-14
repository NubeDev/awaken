//! Describe a datasource's schema for authoring and AI surfaces.
//!
//! Returns the operator-declared [`SchemaBlob`] when the manifest carries one,
//! otherwise introspects `information_schema` under the read-only role and
//! shapes the result into the same [`SchemaBlob`] type (docs "Schema
//! discovery"). Generic over [`SqlBackend`] so the shaping is testable without a
//! live DB.

use crate::backend::SqlBackend;
use crate::error::DatasourceResult;
use crate::manifest::{ColumnSchema, SchemaBlob, TableSchema};

/// Resolve a datasource's schema: the declared blob if present, else live
/// introspection shaped into a blob.
pub async fn describe<B: SqlBackend>(
    backend: &B,
    declared: Option<&SchemaBlob>,
) -> DatasourceResult<SchemaBlob> {
    if let Some(blob) = declared {
        return Ok(blob.clone());
    }
    let raw = backend.introspect().await?;
    Ok(shape_introspection(raw.rows))
}

/// Fold `information_schema.columns` rows — each a JSON cell list of
/// `[table_schema, table_name, column_name, data_type]` — into a [`SchemaBlob`].
/// Rows arrive grouped by table (the query's ORDER BY), so a simple run-length
/// grouping reconstructs each table's columns in order.
fn shape_introspection(rows: Vec<crate::backend::Row>) -> SchemaBlob {
    let mut tables: Vec<TableSchema> = Vec::new();
    for row in rows {
        let schema = cell_str(&row, 0);
        let table = cell_str(&row, 1);
        let column = cell_str(&row, 2);
        let type_name = cell_str(&row, 3);
        let qualified = if schema.is_empty() {
            table.clone()
        } else {
            format!("{schema}.{table}")
        };
        let col = ColumnSchema { name: column, type_name };
        match tables.last_mut() {
            Some(t) if t.name == qualified => t.columns.push(col),
            _ => tables.push(TableSchema {
                name: qualified,
                columns: vec![col],
            }),
        }
    }
    SchemaBlob { tables }
}

/// Read a string cell, defaulting empty for a missing/non-string value.
fn cell_str(row: &[serde_json::Value], i: usize) -> String {
    row.get(i)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::RawResult;
    use crate::error::DatasourceResult;
    use async_trait::async_trait;
    use serde_json::json;

    struct IntrospectBackend(Vec<crate::backend::Row>);

    #[async_trait]
    impl SqlBackend for IntrospectBackend {
        async fn run(
            &self,
            _sql: &str,
            _params: &crate::statement::Params,
            _wall: Option<std::time::Duration>,
            _bound: Option<u64>,
        ) -> DatasourceResult<RawResult> {
            unreachable!("describe uses introspect")
        }
        async fn introspect(&self) -> DatasourceResult<RawResult> {
            Ok(RawResult {
                columns: vec![],
                rows: self.0.clone(),
            })
        }
    }

    #[tokio::test]
    async fn declared_blob_short_circuits_introspection() {
        let blob = SchemaBlob {
            tables: vec![TableSchema {
                name: "readings".into(),
                columns: vec![],
            }],
        };
        let b = IntrospectBackend(vec![]);
        let out = describe(&b, Some(&blob)).await.unwrap();
        assert_eq!(out, blob);
    }

    #[tokio::test]
    async fn introspection_groups_columns_by_qualified_table() {
        let rows = vec![
            vec![json!("public"), json!("readings"), json!("ts"), json!("timestamptz")],
            vec![json!("public"), json!("readings"), json!("v"), json!("float8")],
            vec![json!("public"), json!("sites"), json!("id"), json!("text")],
        ];
        let out = describe(&IntrospectBackend(rows), None).await.unwrap();
        assert_eq!(out.tables.len(), 2);
        assert_eq!(out.tables[0].name, "public.readings");
        assert_eq!(out.tables[0].columns.len(), 2);
        assert_eq!(out.tables[0].columns[1].name, "v");
        assert_eq!(out.tables[1].name, "public.sites");
    }
}
