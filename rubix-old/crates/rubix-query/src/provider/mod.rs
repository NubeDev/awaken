//! A read-only DataFusion `TableProvider` over one SQLite table.
//!
//! Schema is read from `PRAGMA table_info` (so empty tables still expose their
//! columns), and rows are loaded live on each scan, so committed writes are
//! always visible.

mod rows;
mod schema;

use std::any::Any;
use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::Result as DfResult;
use datafusion::datasource::MemTable;
use datafusion::logical_expr::{Expr, TableType};
use datafusion::physical_plan::ExecutionPlan;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::QueryError;

/// A single SQLite table exposed to DataFusion.
#[derive(Debug)]
pub(crate) struct SqliteTable {
    pool: Pool<SqliteConnectionManager>,
    table: String,
    schema: SchemaRef,
}

impl SqliteTable {
    /// Read the table's schema and capture the pool for later scans.
    pub(crate) fn try_new(
        pool: Pool<SqliteConnectionManager>,
        table: &str,
    ) -> Result<Self, QueryError> {
        let conn = pool.get().map_err(|e| QueryError::Pool(e.to_string()))?;
        let schema = schema::table_schema(&conn, table)?;
        Ok(Self {
            pool,
            table: table.to_string(),
            schema,
        })
    }
}

#[async_trait::async_trait]
impl TableProvider for SqliteTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        let conn = self
            .pool
            .get()
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?;
        let batch = rows::read_batch(&conn, &self.table, self.schema.clone())
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?;
        let mem = MemTable::try_new(self.schema.clone(), vec![vec![batch]])?;
        mem.scan(state, projection, filters, limit).await
    }
}
