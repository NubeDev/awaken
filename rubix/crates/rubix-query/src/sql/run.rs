//! Execute a SQL statement and shape the result into JSON rows.

use datafusion::arrow::json::ArrayWriter;

use super::QueryRows;
use crate::context::QueryEngine;
use crate::error::QueryError;

impl QueryEngine {
    /// Run a read-only SQL statement and return its rows as JSON objects.
    ///
    /// The statement is planned and executed by DataFusion against the
    /// registered canonical tables. Result batches are encoded column-wise by
    /// `arrow-json`, preserving nulls and nested types.
    pub async fn query(&self, sql: &str) -> Result<QueryRows, QueryError> {
        let df = self.context().sql(sql).await?;
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
        Ok(serde_json::from_slice(&buf).unwrap_or_default())
    }
}
