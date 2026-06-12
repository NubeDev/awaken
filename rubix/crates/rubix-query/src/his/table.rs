//! A `TableProvider` for `his` that unions the SQLite hot tier with the Parquet
//! cold tier, so `/query` and `/his/rollup` span both transparently.
//!
//! Each scan reads the current SQLite `his` rows and every Parquet partition
//! into one in-memory table. Like the per-table SQLite provider, this favours
//! correctness and live reads over streaming: per-node history is bounded, and
//! the cold tier holds aged rows the hot tier no longer carries.

use std::any::Any;
use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::Result as DfResult;
use datafusion::datasource::MemTable;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{Expr, TableType};
use datafusion::object_store::ObjectStore;
use datafusion::physical_plan::ExecutionPlan;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::hot::read_hot_batch;
use super::read::read_partitions;
use super::schema::his_schema;

/// Unions the SQLite recent tier with Parquet cold partitions under one `his`.
#[derive(Debug)]
pub(crate) struct HisTable {
    pool: Pool<SqliteConnectionManager>,
    store: Arc<dyn ObjectStore>,
    schema: SchemaRef,
}

impl HisTable {
    /// Build the union provider over the SQLite `pool` and the cold-tier `store`.
    pub(crate) fn new(pool: Pool<SqliteConnectionManager>, store: Arc<dyn ObjectStore>) -> Self {
        Self {
            pool,
            store,
            schema: his_schema(),
        }
    }
}

#[async_trait::async_trait]
impl TableProvider for HisTable {
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
        let hot = read_hot_batch(&self.pool).map_err(external)?;
        let mut batches = read_partitions(&self.store).await.map_err(external)?;
        batches.push(hot);
        let mem = MemTable::try_new(self.schema.clone(), vec![batches])?;
        mem.scan(state, projection, filters, limit).await
    }
}

fn external(e: crate::error::QueryError) -> DataFusionError {
    DataFusionError::External(Box::new(e))
}
