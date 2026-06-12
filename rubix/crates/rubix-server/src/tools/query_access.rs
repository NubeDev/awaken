//! [`QueryAccess`] over the DataFusion [`QueryEngine`]: lets the agent `query`
//! tool run read-only SQL against the canonical BMS tables.

use async_trait::async_trait;
use rubix_query::QueryEngine;
use rubix_tools::QueryAccess;

/// Query-engine-backed SQL access handed to the `query` tool.
#[derive(Clone)]
pub struct EngineQueryAccess {
    engine: QueryEngine,
}

impl EngineQueryAccess {
    pub fn new(engine: QueryEngine) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl QueryAccess for EngineQueryAccess {
    async fn query(&self, sql: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        Ok(self.engine.query(sql).await?)
    }
}
