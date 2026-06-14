//! [`QueryAccess`] over the DataFusion [`QueryEngine`]: lets the agent `query`
//! tool run read-only SQL against the canonical BMS tables.
//!
//! An unscoped access runs SQL over every tenant's tables (operator/edge use). A
//! scoped access binds a [`QueryScope`] and runs through
//! [`QueryEngine::scoped_query`], so a tenant-scoped run's SQL can only read its
//! own `{org}/{site}` — the surface that lets a scoped run keep the `query` tool
//! instead of having it withheld.

use async_trait::async_trait;
use rubix_query::{QueryEngine, QueryScope};
use rubix_tools::QueryAccess;

/// Query-engine-backed SQL access handed to the `query` tool, optionally
/// confined to one tenant scope.
#[derive(Clone)]
pub struct EngineQueryAccess {
    engine: QueryEngine,
    scope: Option<QueryScope>,
}

impl EngineQueryAccess {
    /// Unscoped access: SQL runs over every tenant's tables.
    pub fn new(engine: QueryEngine) -> Self {
        Self {
            engine,
            scope: None,
        }
    }

    /// Tenant-scoped access: SQL is confined to `scope`'s `{org}/{site}`.
    pub fn scoped(engine: QueryEngine, scope: QueryScope) -> Self {
        Self {
            engine,
            scope: Some(scope),
        }
    }
}

#[async_trait]
impl QueryAccess for EngineQueryAccess {
    async fn query(&self, sql: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        match &self.scope {
            Some(scope) => Ok(self.engine.scoped_query(scope, sql).await?),
            None => Ok(self.engine.query(sql).await?),
        }
    }
}
