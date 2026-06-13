//! Datasource AI tools: invoke a named query, and describe a datasource.
//!
//! The AI tier is **named-query only** (docs/design/datasources.md "AI"): the
//! agent may invoke an operator-registered named query with bound parameters but
//! may never author raw SQL against a datasource. Raw SQL from a model is a
//! prompt-injection surface; keeping the SQL operator-authored while letting the
//! AI parameterize it is the trust model. Both tools are read-only — the
//! read band, parallel to the `query` tool — and reach the registry through an
//! injected [`DatasourceAccess`] port.

use crate::port::DatasourceAccess;
use crate::prelude::*;

/// Invoke an operator-registered named query on an external datasource.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DatasourceQueryArgs {
    /// The datasource id to read from (see the `describe_datasource` tool).
    pub datasource: String,
    /// The name of an operator-registered named query on that datasource. Raw
    /// SQL is not accepted here — only a registered query name.
    pub query: String,
    /// Positional bound parameters for the query's `$1..$N`, each
    /// `{ "type": "text"|"int"|"float"|"bool"|"timestamp", "value": … }` or
    /// `{ "type": "null" }`. Omit for a parameterless query. Values are bound,
    /// never spliced into SQL.
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// Invokes named datasource queries through an injected [`DatasourceAccess`].
pub struct DatasourceQueryTool {
    access: Arc<dyn DatasourceAccess>,
}

impl DatasourceQueryTool {
    pub fn new(access: Arc<dyn DatasourceAccess>) -> Self {
        Self { access }
    }
}

#[async_trait]
impl TypedTool for DatasourceQueryTool {
    type Args = DatasourceQueryArgs;

    fn tool_id(&self) -> &str {
        "rubix_datasource_query"
    }

    fn name(&self) -> &str {
        "datasource_query"
    }

    fn description(&self) -> &str {
        "Invoke an operator-registered named query on an external SQL datasource \
         (e.g. a TimescaleDB historian) with bound parameters, returning \
         { columns, rows, breached }. You cannot write raw SQL here — only a \
         registered query name. Use describe_datasource to see what a datasource \
         exposes."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let result = self
            .access
            .invoke_named(&args.datasource, &args.query, args.params)
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        Ok(ToolResult::success("datasource_query", result).into())
    }
}

/// Describe the tables and columns an external datasource exposes.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DatasourceDescribeArgs {
    /// The datasource id to describe.
    pub datasource: String,
}

/// Describes a datasource's schema through an injected [`DatasourceAccess`].
pub struct DatasourceDescribeTool {
    access: Arc<dyn DatasourceAccess>,
}

impl DatasourceDescribeTool {
    pub fn new(access: Arc<dyn DatasourceAccess>) -> Self {
        Self { access }
    }
}

#[async_trait]
impl TypedTool for DatasourceDescribeTool {
    type Args = DatasourceDescribeArgs;

    fn tool_id(&self) -> &str {
        "rubix_datasource_describe"
    }

    fn name(&self) -> &str {
        "describe_datasource"
    }

    fn description(&self) -> &str {
        "Return the tables and columns an external SQL datasource exposes \
         ({ tables: [...] }), so you can choose a named query knowingly."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let schema = self
            .access
            .describe(&args.datasource)
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        Ok(ToolResult::success("describe_datasource", schema).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records the last named-query / describe call and returns canned JSON.
    #[derive(Default)]
    struct FakeAccess {
        named: Mutex<Option<(String, String, Vec<serde_json::Value>)>>,
        described: Mutex<Option<String>>,
    }

    #[async_trait]
    impl DatasourceAccess for FakeAccess {
        async fn invoke_named(
            &self,
            datasource: &str,
            name: &str,
            params: Vec<serde_json::Value>,
        ) -> anyhow::Result<serde_json::Value> {
            *self.named.lock().unwrap() =
                Some((datasource.to_string(), name.to_string(), params));
            Ok(serde_json::json!({ "columns": [], "rows": [], "breached": false }))
        }

        async fn describe(&self, datasource: &str) -> anyhow::Result<serde_json::Value> {
            *self.described.lock().unwrap() = Some(datasource.to_string());
            Ok(serde_json::json!({ "tables": [] }))
        }
    }

    #[tokio::test]
    async fn query_tool_forwards_named_query_and_params() {
        let access = Arc::new(FakeAccess::default());
        let tool = DatasourceQueryTool::new(access.clone());
        let out = tool
            .execute(
                DatasourceQueryArgs {
                    datasource: "historian".into(),
                    query: "site_daily".into(),
                    params: vec![serde_json::json!({ "type": "int", "value": 7 })],
                },
                &ToolCallContext::test_default(),
            )
            .await
            .expect("invoke");
        assert_eq!(out.result.data["breached"], serde_json::json!(false));
        let seen = access.named.lock().unwrap().clone().unwrap();
        assert_eq!(seen.0, "historian");
        assert_eq!(seen.1, "site_daily");
        assert_eq!(seen.2.len(), 1);
    }

    /// The tool's arg schema has no `sql` field — the AI cannot author raw SQL
    /// against a datasource, only invoke a registered named query.
    #[test]
    fn query_args_have_no_raw_sql_field() {
        let schema = serde_json::to_value(schemars::schema_for!(DatasourceQueryArgs)).unwrap();
        let props = &schema["properties"];
        assert!(props.get("query").is_some(), "named query field present");
        assert!(props.get("sql").is_none(), "no raw-SQL field on the AI tool");
    }

    #[tokio::test]
    async fn describe_tool_forwards_datasource_id() {
        let access = Arc::new(FakeAccess::default());
        let tool = DatasourceDescribeTool::new(access.clone());
        let out = tool
            .execute(
                DatasourceDescribeArgs {
                    datasource: "historian".into(),
                },
                &ToolCallContext::test_default(),
            )
            .await
            .expect("describe");
        assert!(out.result.data["tables"].is_array());
        assert_eq!(access.described.lock().unwrap().as_deref(), Some("historian"));
    }
}
