//! [`BoardAccess`] over the store: loads a board JSON into a reflow `Network`
//! backed by [`StorePointAccess`] and runs it once. Backs the agent `run_board`
//! tool with the same execution path as `POST /boards/run`.
//!
//! When the run is tenant-scoped, the board's point access is wrapped in
//! [`ScopedPointAccess`] so a board that reads or commands a point outside the
//! run's `{org}/{site}` fails at the point boundary — the agent cannot escape
//! its tenant by routing a write through a board.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_flow::{BoardGraph, PointAccess};
use rubix_tools::{BoardAccess, ScopedPointAccess, TenantScope};

use crate::flow::StorePointAccess;
use crate::store::Store;

/// Store-backed board runner handed to the agent's `run_board` tool. An optional
/// [`TenantScope`] confines the board's point access to one `{org}/{site}`.
pub struct StoreBoardAccess {
    store: Store,
    scope: Option<TenantScope>,
}

impl StoreBoardAccess {
    /// Unscoped board runner: the board may touch any site.
    pub fn new(store: Store) -> Self {
        Self { store, scope: None }
    }

    /// Board runner confined to `scope` when present.
    pub fn scoped(store: Store, scope: Option<TenantScope>) -> Self {
        Self { store, scope }
    }
}

#[async_trait]
impl BoardAccess for StoreBoardAccess {
    async fn run_board(&self, board: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let graph: BoardGraph =
            serde_json::from_value(board).map_err(|e| anyhow::anyhow!("invalid board: {e}"))?;
        let base: Arc<dyn PointAccess> = Arc::new(StorePointAccess::new(self.store.clone()));
        let access: Arc<dyn PointAccess> = match &self.scope {
            Some(scope) => Arc::new(ScopedPointAccess::new(base, scope.clone())),
            None => base,
        };
        let outputs = graph
            .run(access)
            .await
            .map_err(|e| anyhow::anyhow!("run board: {e}"))?;
        Ok(serde_json::json!({ "outputs": outputs }))
    }
}
