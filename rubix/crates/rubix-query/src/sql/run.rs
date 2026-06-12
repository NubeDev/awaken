//! Execute a SQL statement and shape the result into JSON rows.

use datafusion::arrow::json::ArrayWriter;
use datafusion::prelude::SessionContext;

use super::QueryRows;
use crate::context::{QueryEngine, QueryScope};
use crate::error::QueryError;

impl QueryEngine {
    /// Run a read-only SQL statement and return its rows as JSON objects.
    ///
    /// The statement is planned and executed by DataFusion against the
    /// registered canonical tables. Result batches are encoded column-wise by
    /// `arrow-json`, preserving nulls and nested types.
    pub async fn query(&self, sql: &str) -> Result<QueryRows, QueryError> {
        let ctx = self.session().await?;
        run_on(&ctx, sql).await
    }

    /// Run a read-only SQL statement confined to one tenant `scope`.
    ///
    /// Identical to [`query`](Self::query) save that the canonical tables are
    /// tenant-filtered views, so the statement can only read rows under the
    /// scope's `{org}/{site}`. Lets a tenant-scoped agent run ad-hoc SQL without
    /// a cross-tenant read hole.
    pub async fn scoped_query(
        &self,
        scope: &QueryScope,
        sql: &str,
    ) -> Result<QueryRows, QueryError> {
        let ctx = self.scoped_session(scope).await?;
        run_on(&ctx, sql).await
    }
}

/// Plan, execute, and JSON-encode `sql` against a prepared context.
async fn run_on(ctx: &SessionContext, sql: &str) -> Result<QueryRows, QueryError> {
    let df = ctx.sql(sql).await?;
    let batches = df.collect().await?;

    let mut buf = Vec::new();
    {
        let mut writer = ArrayWriter::new(&mut buf);
        for batch in &batches {
            writer.write(batch)?;
        }
        writer.finish()?;
    }

    if buf.is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_slice(&buf)?)
}
