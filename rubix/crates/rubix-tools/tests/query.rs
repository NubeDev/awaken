//! `query` tool: execution path and the read-only gate through the contract.

use std::sync::Arc;

use async_trait::async_trait;
use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolError};
use rubix_tools::{QueryAccess, QueryTool};
use serde_json::json;

/// Echoes a fixed row set; records the SQL it was asked to run.
struct FakeQuery;

#[async_trait]
impl QueryAccess for FakeQuery {
    async fn query(&self, _sql: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        Ok(vec![json!({"slug": "ahu-3"}), json!({"slug": "ahu-4"})])
    }
}

fn tool() -> QueryTool {
    QueryTool::new(Arc::new(FakeQuery))
}

#[tokio::test]
async fn select_returns_rows_with_count() {
    let out = tool()
        .execute(
            json!({ "sql": "SELECT slug FROM equips" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("execute");
    assert!(out.result.is_success());
    assert_eq!(out.result.data["row_count"], json!(2));
    assert_eq!(out.result.data["rows"][0]["slug"], json!("ahu-3"));
}

#[tokio::test]
async fn write_statement_is_denied_before_execution() {
    let err = tool()
        .execute(
            json!({ "sql": "DELETE FROM points" }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::Denied(_)));
}
