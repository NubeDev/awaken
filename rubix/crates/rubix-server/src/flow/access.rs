//! [`PointAccess`] over the SQLite store: the bridge that lets reflow boards
//! read and command points by keyexpr. Writes go through the priority array
//! via [`Store::command_point`]; the agent-priority gate is enforced at the
//! HTTP/tool layer, not here (boards are operator-authored control logic).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use awaken_runtime::run::RunActivation;
use awaken_runtime::AgentRuntime;
use awaken_runtime_contract::contract::message::Message;
use chrono::Utc;
use rubix_core::{HisSample, PointValue, Spark};
use rubix_datasource::{DatasourceRegistry, Param, Params};
use rubix_flow::{
    AgentOutcome, AgentRequest, DatasourceQuery, FlowAccessError, PointAccess, SparkDraft,
};
use uuid::Uuid;

use crate::agent::AGENT_ID;
use crate::bus::ZenohBus;
use crate::flow::TableRuleStore;
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
    /// Tenant scope a `rule` node resolves stored rules through. `None` makes a
    /// stored-rule node fail closed (an inline-script node runs regardless).
    org: Option<String>,
    /// Site slug within the org, when the board acts on one site. A site-scoped
    /// rule resolves first, then the org-level one. `None` → org-level only.
    site: Option<String>,
    /// External datasources a `datasource` node reads through. `None` makes a
    /// `datasource` node fail closed (no datasource manifest loaded, or a board
    /// access — the agent's own `run_board` — that deliberately withholds it).
    datasources: Option<Arc<DatasourceRegistry>>,
}

impl StorePointAccess {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            bus: None,
            agent: None,
            org: None,
            site: None,
            datasources: None,
        }
    }

    /// Construct with a bus so `emit_spark` publishes findings live.
    pub fn with_bus(store: Store, bus: Option<ZenohBus>) -> Self {
        Self {
            store,
            bus,
            agent: None,
            org: None,
            site: None,
            datasources: None,
        }
    }

    /// Add the agent runtime so an `agent_call` node can raise a run. Chained
    /// after `with_bus` on the scheduler/HTTP board paths.
    pub fn with_agent(mut self, agent: Option<Arc<AgentRuntime>>) -> Self {
        self.agent = agent;
        self
    }

    /// Bind the org a `rule` node resolves stored rules and composition in. Set
    /// on the board-run paths from the board's tenant context; absent, a
    /// stored-rule node fails closed.
    pub fn with_org(mut self, org: Option<String>) -> Self {
        self.org = org;
        self
    }

    /// Bind the site a `rule` node resolves a site-scoped rule under (falling
    /// back to the org-level rule). Set from the board's `tenant_site()`; absent,
    /// only org-level rules resolve.
    pub fn with_site(mut self, site: Option<String>) -> Self {
        self.site = site;
        self
    }

    /// Add the datasource registry so a `datasource` node can read external SQL.
    /// Chained on the board-run paths (HTTP `run_board`, the scheduler); absent
    /// (no manifest, or the agent's own board access), a `datasource` node fails
    /// closed.
    pub fn with_datasources(mut self, datasources: Option<Arc<DatasourceRegistry>>) -> Self {
        self.datasources = datasources;
        self
    }
}

#[async_trait]
impl PointAccess for StorePointAccess {
    async fn read_point(&self, keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError> {
        let store = self.store.clone();
        let keyexpr = keyexpr.to_string();
        // SQLite is synchronous; off-load it so the scan loop's cadence reads
        // never park a Tokio worker (G1).
        on_store(move || {
            let id = store.point_by_keyexpr(&keyexpr)?;
            Ok(store.get_point(id)?.cur_value)
        })
        .await
    }

    async fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError> {
        let store = self.store.clone();
        let keyexpr = keyexpr.to_string();
        let now = Utc::now();
        on_store(move || {
            let id = store.point_by_keyexpr(&keyexpr)?;
            Ok(store
                .command_point(id, priority, Some(value), now)?
                .cur_value)
        })
        .await
    }

    async fn query_his(
        &self,
        keyexpr: &str,
        limit: usize,
    ) -> Result<Vec<HisSample>, FlowAccessError> {
        let store = self.store.clone();
        let keyexpr = keyexpr.to_string();
        on_store(move || {
            let id = store.point_by_keyexpr(&keyexpr)?;
            Ok(store.his_query(id, None, None, limit)?)
        })
        .await
    }

    /// Persist a rule-board finding and, when a bus is present, publish it on
    /// `{org}/{site}/spark/{rule}/{id}` so cloud subscribers observe it live —
    /// the same keyexpr scheme as HTTP `POST /sparks`. The port is synchronous;
    /// publishing is detached onto the current runtime (a board only runs inside
    /// one), so a slow or failed publish never blocks the graph. The spark is
    /// already durable, so the publish is best-effort.
    async fn emit_spark(&self, draft: SparkDraft) -> Result<(), FlowAccessError> {
        let store = self.store.clone();
        // `site_prefix` is `{org}/{site}` — split the publish-key parts out before
        // the draft moves into the store task.
        let publish_target = draft
            .site_prefix
            .split_once('/')
            .map(|(org, site)| (org.to_string(), site.to_string()));
        let spark = on_store(move || {
            let site_id = store.site_id_by_prefix(&draft.site_prefix)?;
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
            store.create_spark(&spark)?;
            Ok(spark)
        })
        .await?;
        if let (Some(bus), Some((org, site_slug))) = (&self.bus, publish_target) {
            let bus = bus.clone();
            tokio::spawn(async move {
                bus.publish_spark(&org, &site_slug, &spark).await;
            });
        }
        Ok(())
    }

    /// Raise an agent run for an `agent_call` node. Fails closed when no agent
    /// runtime is wired (notably the `run_board` tool's board access, breaking
    /// recursion). Fire-and-forget: the run is detached onto the current runtime
    /// so the board does not block on the LLM, mirroring `emit_spark`.
    async fn request_agent(&self, request: AgentRequest) -> Result<(), FlowAccessError> {
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

    /// The org-scoped rule store a `rule` node resolves stored rules and
    /// `rule(name, …)` composition through. `None` when no org is bound, so a
    /// stored-rule node fails closed (an inline-script node needs no store). A
    /// pure accessor (no I/O), so it stays synchronous on the async seam.
    fn rule_store(&self) -> Option<Arc<dyn rubix_rules::RuleStore>> {
        self.org.clone().map(|org| {
            Arc::new(TableRuleStore::new(
                self.store.clone(),
                org,
                self.site.clone(),
            )) as Arc<dyn rubix_rules::RuleStore>
        })
    }

    /// Run a `datasource` node's read against the external engine under the
    /// *strict* cap policy and return the `{ columns, rows, breached }` blob.
    /// Fails closed when no datasource registry is wired (no manifest, or the
    /// agent's own board access). Now that the seam is async, the executor is
    /// awaited directly — no `block_in_place`/`block_on` bridge that would park a
    /// Tokio worker for the whole round-trip. Strict: a cap breach is an error
    /// here (the spark path must not fold a truncated grid into a finding — docs
    /// "Truncation on the spark path").
    async fn query_datasource(
        &self,
        datasource: &str,
        query: DatasourceQuery,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, FlowAccessError> {
        let registry = self.datasources.as_ref().ok_or_else(|| {
            FlowAccessError::Unsupported(
                "datasource: no datasource registry on this board access".into(),
            )
        })?;
        let params = parse_params(params).map_err(FlowAccessError::store)?;
        let executor = registry
            .executor(datasource)
            .map_err(FlowAccessError::store)?;
        let run = async {
            match &query {
                DatasourceQuery::Sql(sql) => executor.execute(sql, &params).await,
                DatasourceQuery::Named(name) => executor.invoke_named(name, &params).await,
            }
        };
        // Bound the round-trip so a hung datasource cannot wedge the node task
        // indefinitely on the persistent scan loop.
        let raw = tokio::time::timeout(DATASOURCE_TIMEOUT, run)
            .await
            .map_err(|_| FlowAccessError::Store("datasource query timed out".into()))?
            .map_err(FlowAccessError::store)?;
        // Strict (spark) path: turn a cap breach into an error rather than
        // handing a partial grid downstream.
        let result = executor.strict(raw).map_err(FlowAccessError::store)?;
        serde_json::to_value(result).map_err(FlowAccessError::store)
    }

    /// Run the agent to completion and return its outcome for a board that awaits
    /// the decision. The seam is async, so the run is awaited directly — no
    /// `block_in_place`/`block_on` bridge parking a worker for the LLM round-trip.
    async fn request_agent_awaited(
        &self,
        request: AgentRequest,
    ) -> Result<AgentOutcome, FlowAccessError> {
        let agent = self.agent_or_fail()?.clone();
        // Bound the LLM round-trip so an awaited node can't hang forever.
        let result = tokio::time::timeout(AGENT_TIMEOUT, agent.run_to_completion(activation_for(request)))
            .await
            .map_err(|_| FlowAccessError::Store("agent run timed out".into()))?
            .map_err(|e| FlowAccessError::Store(format!("agent run failed: {e}")))?;
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
    fn agent_or_fail(&self) -> Result<&Arc<AgentRuntime>, FlowAccessError> {
        self.agent.as_ref().ok_or_else(|| {
            FlowAccessError::Unsupported("agent_call: no agent runtime on this board access".into())
        })
    }
}

/// Upper bound on an awaited `agent_call` LLM round-trip before the node fails.
const AGENT_TIMEOUT: Duration = Duration::from_secs(120);
/// Upper bound on a `datasource` node's external query before the node fails.
const DATASOURCE_TIMEOUT: Duration = Duration::from_secs(60);

/// Run a synchronous (SQLite-backed) store closure on the blocking pool and fold
/// both the join error and the store error into a [`FlowAccessError::Store`], so
/// the async seam never blocks a Tokio worker on disk I/O.
async fn on_store<T, F>(f: F) -> Result<T, FlowAccessError>
where
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| FlowAccessError::Store(format!("store task: {e}")))?
        .map_err(FlowAccessError::store)
}

/// Parse a `datasource` node's JSON `params` (a `[{type,value}, …]` array) into
/// the typed [`Params`] the executor binds positionally. A non-array or a
/// malformed entry is a board-authoring error surfaced on the node's `error`
/// port, never spliced into SQL.
fn parse_params(params: serde_json::Value) -> anyhow::Result<Params> {
    let arr = match params {
        serde_json::Value::Array(arr) => arr,
        // A bare `null`/absent params is "no parameters" — the common case for a
        // parameterless query — rather than an error.
        serde_json::Value::Null => Vec::new(),
        other => anyhow::bail!("datasource: `params` must be a JSON array, got {other}"),
    };
    arr.into_iter()
        .map(|v| {
            serde_json::from_value::<Param>(v)
                .map_err(|e| anyhow::anyhow!("datasource: invalid parameter: {e}"))
        })
        .collect()
}

/// Activation for an `agent_call` request: a single user turn on the named
/// thread, run by the embedded [`AGENT_ID`].
fn activation_for(request: AgentRequest) -> RunActivation {
    RunActivation::new(request.thread, vec![Message::user(request.prompt)]).with_agent_id(AGENT_ID)
}
