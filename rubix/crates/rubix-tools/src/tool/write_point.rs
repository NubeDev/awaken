//! `write_point` tool: an agent commands a point through the priority array.
//!
//! Gated per STACK-DEISGN.md: agent writes enter at a configured low priority
//! and may not command at or above (numerically below) the agent-min slot —
//! an operator override always wins. The runtime still enforces tool
//! permission; this gate is the priority-array half. HITL escalation above the
//! threshold (via awaken run suspension) is a later layer.

use rubix_core::PointValue;

use crate::prelude::*;

/// Command a point's priority slot.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WritePointArgs {
    /// Point keyexpr: `{org}/{site}/{equip-path}/{point}`.
    pub point: String,
    /// Value to command: a number, boolean, or string.
    pub value: serde_json::Value,
    /// Priority slot 1..=16 (1 wins). Defaults to the tool's agent priority.
    pub priority: Option<u8>,
}

/// Commands points, confined to agent-eligible priority slots.
pub struct WritePointTool {
    access: Arc<dyn PointAccess>,
    /// Lowest slot number (highest authority) an agent may command. A request
    /// for a slot below this is refused.
    agent_min_priority: u8,
}

impl WritePointTool {
    /// `agent_min_priority` is clamped to 1..=16. Omitted writes default to
    /// slot 16 (lowest authority), which is always agent-eligible.
    pub fn new(access: Arc<dyn PointAccess>, agent_min_priority: u8) -> Self {
        Self {
            access,
            agent_min_priority: agent_min_priority.clamp(1, 16),
        }
    }
}

/// Slot used when the agent omits `priority`: the lowest authority.
const DEFAULT_PRIORITY: u8 = 16;

fn value_of(v: &serde_json::Value) -> Option<PointValue> {
    match v {
        serde_json::Value::Bool(b) => Some(PointValue::Bool(*b)),
        serde_json::Value::Number(n) => n.as_f64().map(PointValue::Number),
        serde_json::Value::String(s) => Some(PointValue::Str(s.clone())),
        _ => None,
    }
}

#[async_trait]
impl TypedTool for WritePointTool {
    type Args = WritePointArgs;

    fn tool_id(&self) -> &str {
        "rubix_write_point"
    }

    fn name(&self) -> &str {
        "write_point"
    }

    fn description(&self) -> &str {
        "Command a BMS point through its 16-level priority array. Agent writes \
         are confined to low-priority slots; an operator override always wins."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let priority = args.priority.unwrap_or(DEFAULT_PRIORITY);
        if !(1..=16).contains(&priority) {
            return Err(ToolError::InvalidArguments(format!(
                "priority must be 1..=16, got {priority}"
            )));
        }
        if priority < self.agent_min_priority {
            return Err(ToolError::Denied(format!(
                "agent writes are limited to priority {} or lower (requested {priority})",
                self.agent_min_priority
            )));
        }
        let Some(value) = value_of(&args.value) else {
            return Err(ToolError::InvalidArguments(
                "value must be a number, boolean, or string".into(),
            ));
        };
        let effective = self
            .access
            .write_point(&args.point, priority, value)
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let data = serde_json::json!({
            "point": args.point,
            "priority": priority,
            "effective": effective,
        });
        Ok(ToolResult::success("write_point", data).into())
    }
}
