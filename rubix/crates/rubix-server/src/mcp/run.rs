//! Execute one external MCP tool call against the gated BMS tool set.
//!
//! An MCP `tools/call` invokes the *same* [`build_tools_scoped`] registry the
//! embedded agent uses, so an external agent gets identical priority-array
//! gating and (WS-07) tenant scoping — there is no path around the gate the
//! tools enforce. The call runs the tool directly (MCP carries the reasoning
//! externally; rubix exposes tools, not a chat loop), so no inference runtime is
//! needed: the adapter builds the tool set from `AppState` and dispatches.
//!
//! A `write_point` that lands in the HITL escalation band suspends rather than
//! committing. The tool returns a *suspended* [`ToolResult`] carrying the held
//! write; this module persists it as a [`RunRecord`] of origin
//! [`RunOrigin::Mcp`] so the externally-initiated run appears in the operator
//! `runs` surface and resumes through the same `POST /runs/{id}/resume` path as
//! a chat or dispatch run. The store is untouched until an operator approves.

use awaken_runtime_contract::contract::identity::RunIdentity;
use awaken_runtime_contract::contract::suspension::ToolCallResume;
use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolResult, ToolStatus};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus, AGENT_ID};
use crate::store::Store;

/// The outcome of an MCP tool call: a finished result, or a suspended write held
/// for operator approval (its run id lets the caller reference the held run).
pub enum CallOutcome {
    /// The tool ran to completion; `result` is the MCP-facing tool result.
    Done(ToolResult),
    /// A `write_point` suspended in the escalation band. The held run was
    /// persisted as `suspended` under `run_id`; an operator approves or cancels
    /// it through the `runs` surface. `result` carries the suspension message.
    Suspended { run_id: String, result: ToolResult },
}

/// Run `tool` with `args`, persisting a suspended write to the `runs` registry.
///
/// `thread_id` keys the external session (an MCP client may reuse it across
/// calls). On a suspended result the held [`PendingWrite`] is recovered from the
/// tool's [`SuspendTicket`] parameters — the same encoding the embedded-agent
/// persist path reads — and stored so the operator surface can re-apply it.
pub async fn run_tool_call(
    store: &Store,
    tool: &dyn Tool,
    args: Value,
    thread_id: &str,
) -> Result<CallOutcome, String> {
    let run_id = Uuid::new_v4().to_string();
    let ctx = context(thread_id, &run_id);
    let output = tool.execute(args, &ctx).await.map_err(|e| e.to_string())?;
    let result = output.result;

    if result.status != ToolStatus::Pending {
        return Ok(CallOutcome::Done(result));
    }

    // Escalation band: the write is held, not applied. Persist the run as
    // suspended with its pending write so the operator surface owns the
    // approve/cancel decision, mirroring `run_and_persist` for chat/dispatch.
    let pending = pending_write_from(&result);
    let now = Utc::now();
    let record = RunRecord {
        id: run_id.clone(),
        thread_id: thread_id.to_string(),
        origin: RunOrigin::Mcp,
        status: RunStatus::Suspended,
        response: result
            .message
            .clone()
            .unwrap_or_else(|| "write awaiting operator approval".to_string()),
        steps: 1,
        pending_write: pending,
        created_at: now,
        updated_at: now,
    };
    if let Err(e) = store.create_run(&record) {
        // A store hiccup must not silently drop the held write — the external
        // caller would believe the command queued. Surface it as a call error.
        return Err(format!("failed to persist suspended MCP run: {e}"));
    }
    Ok(CallOutcome::Suspended { run_id, result })
}

/// Build a minimal tool-call context for a direct (non-loop) MCP invocation. The
/// BMS tools read no loop state, but a faithful [`RunIdentity`] records the MCP
/// origin so the run is attributable to the external session.
fn context(thread_id: &str, run_id: &str) -> ToolCallContext {
    let identity = RunIdentity::new(
        thread_id.to_string(),
        None,
        run_id.to_string(),
        None,
        AGENT_ID.to_string(),
        awaken_runtime_contract::contract::identity::RunOrigin::Mcp,
    );
    ToolCallContext {
        run_identity: identity,
        // `resume_input` stays `None`: this is a fresh call, never a resumed
        // suspension (resume goes through the `runs` HTTP surface, not MCP).
        resume_input: None::<ToolCallResume>,
        ..ToolCallContext::test_default()
    }
}

/// Recover the held write from a suspended `write_point` result's ticket. The
/// tool encodes `{point, priority, value, agent_min_priority}` in its suspension
/// parameters (see `rubix-tools` `write_point`). `None` when the suspension is
/// not a parseable point write — the run still persists as suspended, surfacing
/// as a resume conflict rather than guessing the held command.
fn pending_write_from(result: &ToolResult) -> Option<PendingWrite> {
    let ticket = result.suspension.as_ref()?;
    serde_json::from_value(ticket.suspension.parameters.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use awaken_runtime_contract::contract::suspension::{
        PendingToolCall, SuspendTicket, Suspension,
    };
    use serde_json::json;

    fn suspended_result(params: Value) -> ToolResult {
        let ticket = SuspendTicket::use_decision_as_tool_result(
            Suspension {
                id: String::new(),
                action: "approve_write".into(),
                message: "awaiting approval".into(),
                parameters: params.clone(),
                response_schema: None,
            },
            PendingToolCall::new("", "write_point", params),
        );
        ToolResult::suspended_with("write_point", "awaiting approval", ticket)
    }

    #[test]
    fn pending_write_recovered_from_ticket() {
        let result = suspended_result(json!({
            "point": "nube/hq/ahu-3/fan",
            "priority": 5,
            "value": true,
            "agent_min_priority": 13
        }));
        let pending = pending_write_from(&result).expect("pending write");
        assert_eq!(pending.point, "nube/hq/ahu-3/fan");
        assert_eq!(pending.priority, 5);
    }

    #[test]
    fn no_pending_write_without_ticket() {
        assert!(pending_write_from(&ToolResult::success("write_point", json!({}))).is_none());
    }
}
