//! Ports the tools depend on, implemented by the host. Keeps rubix-tools free
//! of DataFusion (query) and the store/bus — the host wires real backends.
//!
//! Point read/write is the [`rubix_flow::PointAccess`] port, re-exported here
//! so tool consumers have one import surface.

pub use rubix_flow::PointAccess;

use async_trait::async_trait;

/// Read-only SQL access over the canonical BMS tables (`sites`, `equips`,
/// `points`, `his`, `sparks`). The host implements this over the DataFusion
/// query engine; rows come back as JSON objects.
#[async_trait]
pub trait QueryAccess: Send + Sync + 'static {
    /// Run a read-only SQL statement, returning rows as JSON objects.
    async fn query(&self, sql: &str) -> anyhow::Result<Vec<serde_json::Value>>;
}

/// Pin a dashboard widget so a finding or trend an agent surfaced persists on
/// the site dashboard. The host writes it to the store after validating the
/// owning site exists.
#[async_trait]
pub trait WidgetAccess: Send + Sync + 'static {
    /// Pin a widget on `site_id`. `kind` is one of `point_value`,
    /// `point_history`, `board_output`; `target` is a point keyexpr or board
    /// slug per the kind. Returns the new widget id.
    async fn pin_widget(
        &self,
        site_id: uuid::Uuid,
        kind: &str,
        title: &str,
        target: &str,
    ) -> anyhow::Result<uuid::Uuid>;
}

/// Read-only access to external SQL datasources for the AI tier. The AI may
/// only *invoke operator-registered named queries* (never author raw SQL — that
/// would hand a prompt-injection surface raw SQL against a customer's production
/// historian, contradicting the operator-authored trust model;
/// docs/design/datasources.md "AI"). The host implements this over the
/// datasource registry; rows come back in the `{ columns, rows, breached }`
/// shape, schema as the registry's declared/introspected blob.
#[async_trait]
pub trait DatasourceAccess: Send + Sync + 'static {
    /// Invoke a named query on a datasource with positional bound parameters
    /// (`[{type,value}, …]`), returning `{ columns, rows, breached }`. The SQL is
    /// operator-authored; the caller supplies only the datasource id, the query
    /// name, and the parameters.
    async fn invoke_named(
        &self,
        datasource: &str,
        name: &str,
        params: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value>;

    /// The tables and columns a datasource exposes, so the agent can pick a named
    /// query knowingly: the operator-declared schema blob or an
    /// `information_schema` introspection (`{ tables: [...] }`).
    async fn describe(&self, datasource: &str) -> anyhow::Result<serde_json::Value>;
}

/// Run a reflow control/rule board to completion. The host loads the board JSON
/// into a reflow `Network` over the store-backed `PointAccess` and returns each
/// node's outputs. Board writes go through the priority array, so the same
/// gating applies as for direct point commands.
#[async_trait]
pub trait BoardAccess: Send + Sync + 'static {
    /// Evaluate `board` (a [`rubix_flow::BoardGraph`] as JSON) once, returning
    /// the node outputs as JSON.
    async fn run_board(&self, board: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}
