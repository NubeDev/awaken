//! `write_point` tool: an agent commands a point through the priority array.
//!
//! Gated per STACK-DEISGN.md into three bands by slot, where lower numbers are
//! higher authority:
//!
//! - **at/below the agent ceiling** (`priority >= agent_min_priority`): the
//!   write commits immediately.
//! - **escalation band** (`escalation_floor <= priority < agent_min_priority`):
//!   the write is held for human approval — the tool returns a *suspended*
//!   result carrying a [`SuspendTicket`], and awaken's run-suspension model
//!   pauses the run as `Waiting` until an operator resumes or cancels. The
//!   store is untouched until resume.
//! - **below the floor** (`priority < escalation_floor`): operator-reserved
//!   top slots an agent may never command, even with approval — a hard refusal.
//!
//! The runtime still enforces tool permission independently; this is the
//! priority-array half of the gate.

use rubix_core::PointValue;

use awaken_runtime_contract::contract::suspension::{
    PendingToolCall, SuspendTicket, Suspension,
};

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

/// Commands points, confined to agent-eligible priority slots with a
/// human-approval escalation band above the agent ceiling.
pub struct WritePointTool {
    access: Arc<dyn PointAccess>,
    /// Lowest slot number (highest authority) an agent may command outright.
    /// A request above this (numerically below) escalates or is refused.
    agent_min_priority: u8,
    /// Lowest slot reachable *with* human approval. Requests below this are a
    /// hard refusal (operator-reserved top slots). Always `<= agent_min`.
    escalation_floor: u8,
}

impl WritePointTool {
    /// `agent_min_priority` is clamped to 1..=16. Omitted writes default to
    /// slot 16 (lowest authority), which is always agent-eligible. With this
    /// constructor the escalation floor is slot 1: every slot above the ceiling
    /// escalates for approval, none are hard-refused.
    pub fn new(access: Arc<dyn PointAccess>, agent_min_priority: u8) -> Self {
        Self::with_escalation_floor(access, agent_min_priority, 1)
    }

    /// As [`new`](Self::new) but with an explicit operator-reserved floor:
    /// slots below `escalation_floor` are refused even with approval. The floor
    /// is clamped to 1..=`agent_min_priority`.
    pub fn with_escalation_floor(
        access: Arc<dyn PointAccess>,
        agent_min_priority: u8,
        escalation_floor: u8,
    ) -> Self {
        let agent_min_priority = agent_min_priority.clamp(1, 16);
        Self {
            access,
            agent_min_priority,
            escalation_floor: escalation_floor.clamp(1, agent_min_priority),
        }
    }

    /// Build a suspended [`ToolOutput`] holding the requested write for an
    /// operator to approve. The decision payload resolves into the tool result
    /// on resume (`UseDecisionAsToolResult`); the approving surface applies the
    /// write to the store, so the tool itself stays side-effect-free here.
    fn suspend_for_approval(
        &self,
        point: &str,
        priority: u8,
        value: &PointValue,
    ) -> ToolOutput {
        let params = serde_json::json!({
            "point": point,
            "priority": priority,
            "value": value,
            "agent_min_priority": self.agent_min_priority,
        });
        let message = format!(
            "write to {point} at priority {priority} is above the agent ceiling \
             ({}); awaiting operator approval",
            self.agent_min_priority
        );
        let suspension = Suspension {
            id: String::new(),
            action: "approve_write".into(),
            message: message.clone(),
            parameters: params.clone(),
            response_schema: None,
        };
        let pending = PendingToolCall::new("", self.name(), params);
        let ticket = SuspendTicket::use_decision_as_tool_result(suspension, pending);
        ToolResult::suspended_with("write_point", message, ticket).into()
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
        let Some(value) = value_of(&args.value) else {
            return Err(ToolError::InvalidArguments(
                "value must be a number, boolean, or string".into(),
            ));
        };
        if priority < self.escalation_floor {
            return Err(ToolError::Denied(format!(
                "priority {priority} is operator-reserved (below escalation floor {}); \
                 agents may not command it even with approval",
                self.escalation_floor
            )));
        }
        if priority < self.agent_min_priority {
            // Escalation band: hold the write for human approval. The store is
            // not touched; awaken pauses the run until a resume decision lands.
            return Ok(self.suspend_for_approval(&args.point, priority, &value));
        }
        let effective = self
            .access
            .write_point(&args.point, priority, value)
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let data = serde_json::json!({
            "point": args.point,
            "priority": priority,
            "effective": effective,
        });
        Ok(ToolResult::success("write_point", data).into())
    }
}
