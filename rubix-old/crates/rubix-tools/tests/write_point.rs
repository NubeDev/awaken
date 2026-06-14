//! `write_point` tool: priority-array gating and command path.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolError};
use rubix_core::{HisSample, PointValue};
use rubix_flow::{FlowAccessError, PointAccess};
use rubix_tools::WritePointTool;
use serde_json::json;

/// Records the last write so tests can assert the priority that reached the
/// store after gating.
#[derive(Default)]
struct RecordingAccess {
    last: Mutex<Option<(String, u8, PointValue)>>,
}

#[async_trait]
impl PointAccess for RecordingAccess {
    async fn read_point(&self, _keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        Ok(None)
    }
    async fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        *self.last.lock().unwrap() = Some((keyexpr.to_string(), priority, value.clone()));
        Ok(Some(value))
    }
    async fn query_his(
        &self,
        _keyexpr: &str,
        _limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
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
async fn priority_above_agent_min_suspends_for_approval() {
    // Default floor is slot 1, so slot 8 (above the agent ceiling) escalates
    // rather than being denied.
    let (tool, access) = tool();
    let out = tool
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 8 }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("suspended output");
    assert!(out.result.is_pending(), "write should suspend, not commit");
    let ticket = out.result.suspension.expect("suspension ticket");
    assert_eq!(ticket.suspension.action, "approve_write");
    assert_eq!(ticket.suspension.parameters["priority"], json!(8));
    assert_eq!(ticket.suspension.parameters["point"], json!("nube/hq/ahu-3/fan"));
    // The store must never have been touched while awaiting approval.
    assert!(access.last.lock().unwrap().is_none());
}

#[tokio::test]
async fn priority_below_escalation_floor_is_denied() {
    let access = Arc::new(RecordingAccess::default());
    // Slots 1..=2 are operator-reserved; agent ceiling at 13.
    let tool = WritePointTool::with_escalation_floor(access.clone(), 13, 3);
    let err = tool
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 2 }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::Denied(_)));
    // A slot inside the escalation band still suspends, not denies.
    let out = tool
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 8 }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("suspended");
    assert!(out.result.is_pending());
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
