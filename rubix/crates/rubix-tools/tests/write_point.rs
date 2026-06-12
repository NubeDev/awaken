//! `write_point` tool: priority-array gating and command path.

use std::sync::Arc;
use std::sync::Mutex;

use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolError};
use rubix_core::{HisSample, PointValue};
use rubix_flow::PointAccess;
use rubix_tools::WritePointTool;
use serde_json::json;

/// Records the last write so tests can assert the priority that reached the
/// store after gating.
#[derive(Default)]
struct RecordingAccess {
    last: Mutex<Option<(String, u8, PointValue)>>,
}

impl PointAccess for RecordingAccess {
    fn read_point(&self, _keyexpr: &str) -> anyhow::Result<Option<PointValue>> {
        Ok(None)
    }
    fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>> {
        *self.last.lock().unwrap() = Some((keyexpr.to_string(), priority, value.clone()));
        Ok(Some(value))
    }
    fn query_his(&self, _keyexpr: &str, _limit: usize) -> anyhow::Result<Vec<HisSample>> {
        Ok(vec![])
    }
}

/// agent_min_priority = 13: agent may command slots 13..=16, not 1..=12.
fn tool() -> (WritePointTool, Arc<RecordingAccess>) {
    let access = Arc::new(RecordingAccess::default());
    (WritePointTool::new(access.clone(), 13), access)
}

#[tokio::test]
async fn omitted_priority_defaults_to_lowest_slot() {
    let (tool, access) = tool();
    let out = tool
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("execute");
    assert!(out.result.is_success());
    let (key, priority, value) = access.last.lock().unwrap().clone().unwrap();
    assert_eq!(key, "nube/hq/ahu-3/fan");
    assert_eq!(priority, 16);
    assert_eq!(value, PointValue::Bool(true));
}

#[tokio::test]
async fn agent_eligible_priority_is_allowed() {
    let (tool, access) = tool();
    tool.execute(
        json!({ "point": "nube/hq/ahu-3/fan", "value": 1.0, "priority": 13 }),
        &ToolCallContext::test_default(),
    )
    .await
    .expect("execute");
    assert_eq!(access.last.lock().unwrap().as_ref().unwrap().1, 13);
}

#[tokio::test]
async fn priority_above_agent_min_is_denied() {
    let (tool, access) = tool();
    let err = tool
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 8 }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::Denied(_)));
    // The store must never have been touched.
    assert!(access.last.lock().unwrap().is_none());
}

#[tokio::test]
async fn out_of_range_priority_is_invalid() {
    let (tool, _) = tool();
    let err = tool
        .execute(
            json!({ "point": "p", "value": true, "priority": 0 }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::InvalidArguments(_)));
}

#[tokio::test]
async fn non_scalar_value_is_invalid() {
    let (tool, _) = tool();
    let err = tool
        .execute(
            json!({ "point": "p", "value": {"nested": 1} }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::InvalidArguments(_)));
}
