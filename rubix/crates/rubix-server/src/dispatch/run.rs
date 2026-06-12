//! Activate one agent run for a received spark. Decodes the published finding,
//! renders the job prompt, and runs the embedded agent to completion on the
//! spark's thread. Outcomes are logged — a dispatched run's effect is its tool
//! calls (reads, history, gated commands), not a returned value. A run that
//! suspends for human approval is logged with its run id so an operator surface
//! can resume it.

use std::sync::Arc;

use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::message::Message;
use rubix_core::Spark;

use super::job;
use crate::agent::{run_and_persist, RunOrigin, RunStatus, AGENT_ID};
use crate::store::Store;

/// Decode a published spark payload and run the agent on it. The run is
/// persisted (a suspended finding lands on the operator surface, an in-flight
/// dispatched run is no longer fire-and-log). Never panics; a decode or run
/// failure is logged so one bad finding cannot stop the loop.
pub(super) async fn dispatch_spark(payload: &[u8], runtime: &Arc<AgentRuntime>, store: &Store) {
    let spark: Spark = match serde_json::from_slice(payload) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "dispatch: undecodable spark payload; skipping");
            return;
        }
    };
    let thread = job::thread_id(&spark);
    let activation =
        RunActivation::new(thread.clone(), vec![Message::user(job::prompt(&spark))])
            .with_agent_id(AGENT_ID);
    match run_and_persist(runtime, store, RunOrigin::Dispatch, activation).await {
        Ok(record) => match record.status {
            RunStatus::Suspended => {
                tracing::info!(
                    rule = %spark.rule, run_id = %record.id,
                    "dispatch: agent run suspended for approval"
                );
            }
            _ => {
                tracing::info!(
                    rule = %spark.rule, steps = record.steps,
                    "dispatch: agent run completed"
                );
            }
        },
        Err(e) => {
            tracing::warn!(rule = %spark.rule, error = %e, "dispatch: agent run failed");
        }
    }
}
