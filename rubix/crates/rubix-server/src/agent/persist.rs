//! Run an agent activation and persist the outcome as a [`RunRecord`].
//!
//! `AgentRunResult` carries only the run id, response, step count, and
//! termination — not the suspended tool call's held write. So the run is driven
//! through [`AgentRuntime::run`] with a collecting [`EventSink`]; on a
//! `Suspended` termination the held [`PendingWrite`] is recovered from the
//! `write_point` tool's `ToolCallDone` event (its `SuspendTicket` parameters).
//! Both the chat endpoint and inbound dispatch persist their runs this way, so
//! a suspended run survives the request that raised it and the operator surface
//! (list / get / resume / cancel) has a row to act on.

use std::sync::Arc;

use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::event::AgentEvent;
use awaken_runtime_contract::contract::event_sink::VecEventSink;
use awaken_runtime_contract::contract::lifecycle::TerminationReason;
use awaken_runtime_contract::contract::suspension::ToolCallOutcome;
use chrono::Utc;

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use crate::store::Store;

/// The tool name a held BMS write surfaces under in its `ToolCallDone` event.
const WRITE_TOOL: &str = "write_point";

/// Drive `activation` to completion, persist the resulting [`RunRecord`], and
/// return it. A suspended run's held write is captured and stored so the resume
/// endpoint can re-apply it. Persisting the row is best-effort logged on failure
/// — a store hiccup must not lose the agent's response to the caller.
pub async fn run_and_persist(
    runtime: &Arc<AgentRuntime>,
    store: &Store,
    origin: RunOrigin,
    activation: RunActivation,
) -> Result<RunRecord, awaken_runtime::loop_runner::AgentLoopError> {
    let thread_id = activation.thread_id().to_string();
    let sink = Arc::new(VecEventSink::new());
    let result = runtime.run(activation, sink.clone()).await?;

    let status = match result.termination {
        TerminationReason::Suspended => RunStatus::Suspended,
        _ => RunStatus::Completed,
    };
    let pending_write = match status {
        RunStatus::Suspended => pending_write_from_events(&sink.take()),
        _ => None,
    };
    let now = Utc::now();
    let record = RunRecord {
        id: result.run_id,
        thread_id,
        origin,
        status,
        response: result.response,
        steps: result.steps,
        pending_write,
        created_at: now,
        updated_at: now,
    };
    if let Err(e) = store.create_run(&record) {
        tracing::error!(run_id = %record.id, error = %e, "failed to persist agent run");
    }
    Ok(record)
}

/// Recover the held write from the last suspended `write_point` tool call in the
/// event stream. The tool encodes `{point, priority, value, agent_min_priority}`
/// in its suspension parameters (see `rubix-tools` `write_point`). Returns
/// `None` if the suspension was not a point write or the parameters are
/// malformed — the run still persists as suspended, just without a re-appliable
/// write, which the resume endpoint surfaces as a conflict rather than guessing.
fn pending_write_from_events(events: &[AgentEvent]) -> Option<PendingWrite> {
    events.iter().rev().find_map(|event| match event {
        AgentEvent::ToolCallDone {
            result,
            outcome: ToolCallOutcome::Suspended,
            ..
        } if result.tool_name == WRITE_TOOL => result
            .suspension
            .as_ref()
            .and_then(|ticket| serde_json::from_value(ticket.suspension.parameters.clone()).ok()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use awaken_runtime_contract::contract::suspension::{
        PendingToolCall, SuspendTicket, Suspension,
    };
    use awaken_runtime_contract::contract::tool::ToolResult;
    use rubix_core::PointValue;
    use serde_json::json;

    fn suspended_write_event(params: serde_json::Value) -> AgentEvent {
        let ticket = SuspendTicket::use_decision_as_tool_result(
            Suspension {
                id: String::new(),
                action: "approve_write".into(),
                message: "awaiting approval".into(),
                parameters: params.clone(),
                response_schema: None,
            },
            PendingToolCall::new("", "write_point", params.clone()),
        );
        AgentEvent::ToolCallDone {
            id: "c1".into(),
            message_id: "m1".into(),
            result: ToolResult::suspended_with("write_point", "awaiting approval", ticket),
            outcome: ToolCallOutcome::Suspended,
        }
    }

    #[test]
    fn recovers_pending_write_from_suspended_event() {
        let events = vec![suspended_write_event(json!({
            "point": "nube/hq/ahu-3/fan",
            "priority": 5,
            "value": true,
            "agent_min_priority": 13
        }))];
        let pending = pending_write_from_events(&events).expect("pending write");
        assert_eq!(pending.point, "nube/hq/ahu-3/fan");
        assert_eq!(pending.priority, 5);
        assert_eq!(pending.value, PointValue::Bool(true));
        assert_eq!(pending.agent_min_priority, 13);
    }

    #[test]
    fn no_pending_write_without_a_suspended_write_event() {
        assert!(pending_write_from_events(&[]).is_none());
        let succeeded = AgentEvent::ToolCallDone {
            id: "c1".into(),
            message_id: "m1".into(),
            result: ToolResult::success("write_point", json!({})),
            outcome: ToolCallOutcome::Succeeded,
        };
        assert!(pending_write_from_events(&[succeeded]).is_none());
    }
}
