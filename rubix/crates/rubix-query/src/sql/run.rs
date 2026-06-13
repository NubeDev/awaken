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
        ensure_read_only(sql)?;
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
        ensure_read_only(sql)?;
        let ctx = self.scoped_session(scope).await?;
        run_on(&ctx, sql).await
    }
}

/// Accept only a single read-only `SELECT`/`WITH` statement; reject writes,
/// DDL, and multi-statement input with [`QueryError::NotReadOnly`].
///
/// Conservative by design: one statement, leading keyword `SELECT` or `WITH`,
/// no statement separator. The DataFusion providers are themselves read-only
/// (a `DROP`/`INSERT` no-ops rather than mutating the store), but refusing
/// writes here gives every caller — the HTTP `/query` route and the agent
/// `query` tool alike — a clear error instead of an empty result or an obscure
/// planning failure, and defends against a future mutable provider.
pub fn ensure_read_only(sql: &str) -> Result<(), QueryError> {
    let trimmed = sql.trim().trim_end_matches(';');
    if trimmed.contains(';') {
        return Err(QueryError::NotReadOnly); // multiple statements
    }
    let lead = trimmed
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(lead.as_str(), "SELECT" | "WITH") {
        Ok(())
    } else {
        Err(QueryError::NotReadOnly)
    }
}

#[cfg(test)]
mod read_only_tests {
    use super::ensure_read_only;

    #[test]
    fn accepts_single_select_or_with() {
        assert!(ensure_read_only("SELECT * FROM points").is_ok());
        assert!(ensure_read_only("  select 1").is_ok());
        assert!(ensure_read_only("WITH t AS (SELECT 1) SELECT * FROM t").is_ok());
        assert!(ensure_read_only("SELECT * FROM points;").is_ok());
    }

    #[test]
    fn rejects_writes_ddl_and_multi_statement() {
        assert!(ensure_read_only("INSERT INTO points VALUES (1)").is_err());
        assert!(ensure_read_only("UPDATE points SET x = 1").is_err());
        assert!(ensure_read_only("DELETE FROM points").is_err());
        assert!(ensure_read_only("DROP TABLE his").is_err());
        assert!(ensure_read_only("SELECT 1; DROP TABLE points").is_err());
        assert!(ensure_read_only("SELECT 1; SELECT 2").is_err());
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
