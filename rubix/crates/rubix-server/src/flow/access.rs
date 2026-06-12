//! [`PointAccess`] over the SQLite store: the bridge that lets reflow boards
//! read and command points by keyexpr. Writes go through the priority array
//! via [`Store::command_point`]; the agent-priority gate is enforced at the
//! HTTP/tool layer, not here (boards are operator-authored control logic).

use std::sync::Arc;

use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::message::Message;
use chrono::Utc;
use rubix_core::{HisSample, PointValue, Spark};
use rubix_flow::{AgentOutcome, AgentRequest, PointAccess, SparkDraft};
use uuid::Uuid;

use crate::agent::AGENT_ID;
use crate::bus::ZenohBus;
use crate::store::Store;

/// Store-backed point access handed to [`rubix_flow::BoardGraph::load`]. An
/// optional bus lets board-emitted sparks publish on their rule keyexpr, the
/// same way HTTP `POST /sparks` does. An optional agent runtime lets an
/// `agent_call` node activate a run. Both are absent for the board access the
/// agent's own `run_board` tool builds — so `agent_call` fails closed there and
/// the agent → board → agent loop cannot recur.
#[derive(Clone)]
pub struct StorePointAccess {
    store: Store,
    bus: Option<ZenohBus>,
    agent: Option<Arc<AgentRuntime>>,
}

impl StorePointAccess {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            bus: None,
            agent: None,
        }
    }

    /// Construct with a bus so `emit_spark` publishes findings live.
    pub fn with_bus(store: Store, bus: Option<ZenohBus>) -> Self {
        Self {
            store,
            bus,
            agent: None,
        }
    }

    /// Add the agent runtime so an `agent_call` node can raise a run. Chained
    /// after `with_bus` on the scheduler/HTTP board paths.
    pub fn with_agent(mut self, agent: Option<Arc<AgentRuntime>>) -> Self {
        self.agent = agent;
        self
    }
}

impl PointAccess for StorePointAccess {
    fn read_point(&self, keyexpr: &str) -> anyhow::Result<Option<PointValue>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        Ok(self.store.get_point(id)?.cur_value)
    }

    fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        let point = self
            .store
            .command_point(id, priority, Some(value), Utc::now())?;
        Ok(point.cur_value)
    }

    fn query_his(&self, keyexpr: &str, limit: usize) -> anyhow::Result<Vec<HisSample>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        Ok(self.store.his_query(id, None, None, limit)?)
    }

    /// Persist a rule-board finding and, when a bus is present, publish it on
    /// `{org}/{site}/spark/{rule}/{id}` so cloud subscribers observe it live —
    /// the same keyexpr scheme as HTTP `POST /sparks`. The port is synchronous;
    /// publishing is detached onto the current runtime (a board only runs inside
    /// one), so a slow or failed publish never blocks the graph. The spark is
    /// already durable, so the publish is best-effort.
    fn emit_spark(&self, draft: SparkDraft) -> anyhow::Result<()> {
        let site_id = self.store.site_id_by_prefix(&draft.site_prefix)?;
        let spark = Spark {
            id: Uuid::new_v4(),
            site_id,
            rule: draft.rule,
            severity: draft.severity,
            message: draft.message,
            point_ids: Vec::new(),
            ts: Utc::now(),
            acknowledged: false,
        };
        self.store.create_spark(&spark)?;
        if let Some(bus) = &self.bus {
            // `site_prefix` is `{org}/{site}` — the two segments the publish key
            // needs. Resolved already by `site_id_by_prefix`, so it is well-formed.
            if let Some((org, site_slug)) = draft.site_prefix.split_once('/') {
                let bus = bus.clone();
                let (org, site_slug) = (org.to_string(), site_slug.to_string());
                tokio::spawn(async move {
                    bus.publish_spark(&org, &site_slug, &spark).await;
                });
            }
        }
        Ok(())
    }

    /// Raise an agent run for an `agent_call` node. Fails closed when no agent
    /// runtime is wired (notably the `run_board` tool's board access, breaking
    /// recursion). Fire-and-forget: the run is detached onto the current runtime
    /// so the board does not block on the LLM, mirroring `emit_spark`.
    fn request_agent(&self, request: AgentRequest) -> anyhow::Result<()> {
        let agent = self.agent_or_fail()?.clone();
        tokio::spawn(async move {
            match agent.run_to_completion(activation_for(request)).await {
                Ok(result) => {
                    tracing::info!(steps = result.steps, "agent_call run completed");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "agent_call run failed");
                }
            }
        });
        Ok(())
    }

    /// Run the agent to completion and return its outcome for a board that awaits
    /// the decision. The port is synchronous and a board only ever runs inside a
    /// Tokio runtime, so we bridge to async with `block_in_place` +
    /// `block_on` — the supported way to await on a multi-thread runtime worker
    /// without starving it (a current-thread runtime would panic, which is the
    /// correct fail-fast: an awaited `agent_call` needs the multi-thread flavor).
    fn request_agent_blocking(&self, request: AgentRequest) -> anyhow::Result<AgentOutcome> {
        let agent = self.agent_or_fail()?.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async move { agent.run_to_completion(activation_for(request)).await })
        })
        .map_err(|e| anyhow::anyhow!("agent run failed: {e}"))?;
        Ok(AgentOutcome {
            run_id: result.run_id,
            response: result.response,
            steps: result.steps,
        })
    }
}

impl StorePointAccess {
    /// The wired agent runtime, or the fail-closed error that breaks the
    /// agent → board → agent recursion when none is present.
    fn agent_or_fail(&self) -> anyhow::Result<&Arc<AgentRuntime>> {
        self.agent
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("agent_call: no agent runtime on this board access"))
    }
}

/// Activation for an `agent_call` request: a single user turn on the named
/// thread, run by the embedded [`AGENT_ID`].
fn activation_for(request: AgentRequest) -> RunActivation {
    RunActivation::new(request.thread, vec![Message::user(request.prompt)]).with_agent_id(AGENT_ID)
}
