//! `read_point` tool: descriptor schema and execution through the awaken
//! `Tool` contract (via the blanket `TypedTool` impl).

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext};
use rubix_core::{HisSample, PointValue};
use rubix_flow::PointAccess;
use rubix_tools::ReadPointTool;
use serde_json::json;

/// Returns a fixed value for any keyexpr; errors for the sentinel "missing".
struct FakeAccess;

impl PointAccess for FakeAccess {
    fn read_point(&self, keyexpr: &str) -> anyhow::Result<Option<PointValue>> {
        if keyexpr == "no/such/point/here" {
            anyhow::bail!("point not found");
        }
        Ok(Some(PointValue::Number(21.5)))
    }
    fn write_point(
        &self,
        _keyexpr: &str,
        _priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>> {
        Ok(Some(value))
    }
    fn query_his(&self, _keyexpr: &str, _limit: usize) -> anyhow::Result<Vec<HisSample>> {
        Ok(vec![])
    }
}

fn tool() -> ReadPointTool {
    ReadPointTool::new(Arc::new(FakeAccess))
}

#[test]
fn descriptor_advertises_point_argument() {
    let desc = tool().descriptor();
    assert_eq!(desc.id, "rubix_read_point");
    assert_eq!(desc.category.as_deref(), Some("bms"));
    // The schema must require the `point` keyexpr argument.
    let props = &desc.parameters["properties"];
    assert!(props.get("point").is_some(), "schema: {}", desc.parameters);
}

#[tokio::test]
async fn reads_current_value() {
    let out = tool()
        .execute(
            json!({ "point": "nube/hq/ahu-3/temp" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("execute");
    assert!(out.result.is_success());
    assert_eq!(out.result.data["value"], json!(21.5));
    assert_eq!(out.result.data["point"], json!("nube/hq/ahu-3/temp"));
}

#[tokio::test]
async fn missing_argument_is_invalid() {
    let err = tool()
        .execute(json!({}), &ToolCallContext::test_default())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        awaken_runtime_contract::contract::tool::ToolError::InvalidArguments(_)
    ));
}

#[tokio::test]
async fn access_error_surfaces_as_internal() {
    let err = tool()
        .execute(
            json!({ "point": "no/such/point/here" }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        awaken_runtime_contract::contract::tool::ToolError::Internal(_)
    ));
}
