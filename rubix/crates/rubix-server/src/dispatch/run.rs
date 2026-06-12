//! Activate one agent run for a received spark. Decodes the published finding,
//! renders the job prompt, and runs the embedded agent to completion on the
//! spark's thread. Outcomes are logged — a dispatched run's effect is its tool
//! calls (reads, history, gated commands), not a returned value. A run that
//! suspends for human approval is logged with its run id so an operator surface
//! can resume it.

use std::sync::Arc;

use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::lifecycle::TerminationReason;
use awaken_runtime_contract::contract::message::Message;
use rubix_core::Spark;

use super::job;
use crate::agent::AGENT_ID;

/// Decode a published spark payload and run the agent on it. Never panics; a
/// decode or run failure is logged so one bad finding cannot stop the loop.
pub(super) async fn dispatch_spark(payload: &[u8], runtime: &Arc<AgentRuntime>) {
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
    match runtime.run_to_completion(activation).await {
        Ok(result) => match result.termination {
            TerminationReason::Suspended => {
                tracing::info!(
                    rule = %spark.rule, run_id = %result.run_id,
                    "dispatch: agent run suspended for approval"
                );
            }
            _ => {
                tracing::info!(
                    rule = %spark.rule, steps = result.steps,
                    "dispatch: agent run completed"
                );
            }
        },
        Err(e) => {
            tracing::warn!(rule = %spark.rule, error = %e, "dispatch: agent run failed");
        }
    }
}
