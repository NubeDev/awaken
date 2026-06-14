//! `query` tool: an agent runs read-only SQL over the canonical BMS tables.
//!
//! Read-only is enforced here, not just by convention: only a single
//! `SELECT`/`WITH` statement is accepted. The DataFusion surface is itself
//! read-only, but rejecting writes up front gives the agent a clear error and
//! defends against a future mutable provider.

use crate::port::QueryAccess;
use crate::prelude::*;

/// Run a read-only SQL query over the BMS tables.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryArgs {
    /// A single read-only `SELECT` (or `WITH … SELECT`) statement. Tables:
    /// `sites`, `equips`, `points`, `his`, `sparks`.
    pub sql: String,
}

/// Runs SQL through an injected [`QueryAccess`].
pub struct QueryTool {
    access: Arc<dyn QueryAccess>,
}

impl QueryTool {
    pub fn new(access: Arc<dyn QueryAccess>) -> Self {
        Self { access }
    }
}

/// Reject anything that isn't a single read-only statement. Conservative: one
/// statement, starting with SELECT or WITH, no statement separator.
fn is_read_only(sql: &str) -> bool {
    let trimmed = sql.trim().trim_end_matches(';');
    if trimmed.contains(';') {
        return false; // multiple statements
    }
    let lead = trimmed
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(lead.as_str(), "SELECT" | "WITH")
}

#[async_trait]
impl TypedTool for QueryTool {
    type Args = QueryArgs;

    fn tool_id(&self) -> &str {
        "rubix_query"
    }

    fn name(&self) -> &str {
        "query"
    }

    fn description(&self) -> &str {
        "Run a read-only SQL SELECT over the BMS tables (sites, equips, points, \
         his, sparks) and return matching rows."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        if !is_read_only(&args.sql) {
            return Err(ToolError::Denied(
                "only a single read-only SELECT/WITH statement is allowed".into(),
            ));
        }
        let rows = self
            .access
            .query(&args.sql)
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let data = serde_json::json!({ "row_count": rows.len(), "rows": rows });
        Ok(ToolResult::success("query", data).into())
    }
}

#[cfg(test)]
mod tests {
    use super::is_read_only;

    #[test]
    fn accepts_select_and_with() {
        assert!(is_read_only("SELECT * FROM points"));
        assert!(is_read_only("  select 1"));
        assert!(is_read_only("WITH t AS (SELECT 1) SELECT * FROM t"));
        assert!(is_read_only("SELECT * FROM points;"));
    }

    #[test]
    fn rejects_writes_and_multi_statement() {
        assert!(!is_read_only("INSERT INTO points VALUES (1)"));
        assert!(!is_read_only("UPDATE points SET x = 1"));
        assert!(!is_read_only("DELETE FROM points"));
        assert!(!is_read_only("DROP TABLE points"));
        assert!(!is_read_only("SELECT 1; DROP TABLE points"));
    }
}
