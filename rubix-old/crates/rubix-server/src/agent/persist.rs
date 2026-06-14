//! Run an agent activation and persist the outcome as a [`RunRecord`].
//!
//! `AgentRunResult` carries only the run id, response, step count, and
//! termination — not the suspended tool call's held write. So the run is driven
//! through [`AgentRuntime::run`] with a [`SuspendCaptureSink`]; on a `Suspended`
//! termination the held [`PendingWrite`] is recovered from the `write_point`
//! tool's `ToolCallDone` event (its `SuspendTicket` parameters).
//!
//! The embedded full-local backend blocks the agent loop after suspending,
//! waiting for an operator decision on a live channel. This architecture feeds
//! no decision back into the loop — the operator surface is the `runs` HTTP API,
//! which re-applies the held write itself. So on the suspend signal the run is
//! cancelled to release the loop; the persisted [`RunRecord`] (status
//! `suspended`, with the held write) is the durable source of truth the operator
//! acts on. Both the chat endpoint and inbound dispatch persist runs this way.

use std::sync::Arc;

use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::event::AgentEvent;
use awaken_runtime_contract::contract::suspension::ToolCallOutcome;
use chrono::Utc;

use super::capture_sink::SuspendCaptureSink;
use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use crate::store::Store;

/// The tool name a held BMS write surfaces under in its `ToolCallDone` event.
const WRITE_TOOL: &str = "write_point";

/// Drive `activation` to completion (or to suspension), persist the resulting
/// [`RunRecord`], and return it. A suspended run is cancelled to release the
/// blocked loop after its held write is captured; the record persists as
/// `suspended` so the resume endpoint can re-apply it. Persisting the row is
/// best-effort logged on failure — a store hiccup must not lose the agent's
/// response to the caller.
pub async fn run_and_persist(
    runtime: &Arc<AgentRuntime>,
    store: &Store,
    origin: RunOrigin,
    activation: RunActivation,
) -> Result<RunRecord, awaken_runtime::loop_runner::AgentLoopError> {
    let thread_id = activation.thread_id().to_string();
    let sink = SuspendCaptureSink::new();
    let run_runtime = runtime.clone();
    let run_sink = sink.clone();
    let mut run = tokio::spawn(async move { run_runtime.run(activation, run_sink).await });

    // Race the run against its own suspend signal. The full-local backend blocks
    // the loop after emitting `RunFinish { Suspended }`; cancelling by run id
    // unblocks it (the wait observes the cooperative cancellation token).
    let suspended = tokio::select! {
        result = &mut run => return finish(result, store, origin, thread_id, &sink, false),
        () = sink.suspended() => true,
    };
    if let Some(run_id) = sink.suspended_run_id() {
        runtime.cancel_by_run_id(&run_id);
    }
    let result = (&mut run).await;
    finish(result, store, origin, thread_id, &sink, suspended)
}

/// Build and persist the record from the joined run result. `suspended` carries
/// whether the suspend signal fired (the run was cancelled after suspending), so
/// the cancelled termination is recorded as `suspended` — the operator-actionable
/// state — rather than `cancelled`.
fn finish(
    joined: Result<
        Result<
            awaken_runtime::loop_runner::AgentRunResult,
            awaken_runtime::loop_runner::AgentLoopError,
        >,
        tokio::task::JoinError,
    >,
    store: &Store,
    origin: RunOrigin,
    thread_id: String,
    sink: &SuspendCaptureSink,
    suspended: bool,
) -> Result<RunRecord, awaken_runtime::loop_runner::AgentLoopError> {
    let result = joined.map_err(|e| {
        awaken_runtime::loop_runner::AgentLoopError::InferenceFailed(format!("run task join: {e}"))
    })??;
    let events = sink.take_events();
    let (status, pending_write) = if suspended {
        (RunStatus::Suspended, pending_write_from_events(&events))
    } else {
        (RunStatus::Completed, None)
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
