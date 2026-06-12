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
