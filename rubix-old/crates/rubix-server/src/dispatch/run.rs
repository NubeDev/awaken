//! Activate one agent run for a received spark. Decodes the published finding,
//! renders the job prompt, and runs the embedded agent to completion on the
//! spark's thread. Outcomes are logged — a dispatched run's effect is its tool
//! calls (reads, history, gated commands), not a returned value. A run that
//! suspends for human approval is logged with its run id so an operator surface
//! can resume it.

use awaken_runtime::run::RunActivation;
use awaken_runtime_contract::contract::message::Message;
use rubix_core::Spark;
use rubix_tools::TenantScope;

use super::job;
use crate::agent::{run_and_persist, runtime_for_scope, RunOrigin, RunStatus, AGENT_ID};
use crate::AppState;

/// Decode a published spark payload and run the agent on it, confined to the
/// spark's tenant. The run is persisted (a suspended finding lands on the
/// operator surface, an in-flight dispatched run is no longer fire-and-log).
/// Never panics; a decode, scope-resolution, or run failure is logged so one bad
/// finding cannot stop the loop.
pub(super) async fn dispatch_spark(payload: &[u8], state: &AppState) {
    let spark: Spark = match serde_json::from_slice(payload) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "dispatch: undecodable spark payload; skipping");
            return;
        }
    };
    // Confine the run to the finding's tenant. The spark names its site; resolve
    // it to `{org}/{site}` so the run's tools cannot reach another tenant. A site
    // that no longer exists fails closed — the run is skipped, not run unscoped.
    let scope = match spark_scope(state, &spark) {
        Ok(scope) => scope,
        Err(e) => {
            tracing::warn!(rule = %spark.rule, error = %e, "dispatch: cannot resolve spark tenant; skipping");
            return;
        }
    };
    let runtime = match runtime_for_scope(state, Some(scope)) {
        Ok(Some(runtime)) => runtime,
        Ok(None) => {
            tracing::warn!(rule = %spark.rule, "dispatch: agent runtime unavailable; skipping");
            return;
        }
        Err(e) => {
            tracing::warn!(rule = %spark.rule, error = %e, "dispatch: cannot build scoped agent; skipping");
            return;
        }
    };
    let thread = job::thread_id(&spark);
    let activation = RunActivation::new(thread.clone(), vec![Message::user(job::prompt(&spark))])
        .with_agent_id(AGENT_ID);
    match run_and_persist(&runtime, &state.store, RunOrigin::Dispatch, activation).await {
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

/// Resolve the spark's owning site to the `{org}/{site}` tenant the run is
/// confined to. Fails when the site is unknown — the run is skipped rather than
/// run with cross-tenant reach.
fn spark_scope(state: &AppState, spark: &Spark) -> anyhow::Result<TenantScope> {
    let site = state.store.get_site(spark.site_id)?;
    Ok(TenantScope::new(site.org, site.slug))
}
