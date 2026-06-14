//! [`DatasourceAccess`] over the [`DatasourceRegistry`]: lets the agent's
//! `datasource_query` / `describe_datasource` tools read external SQL.
//!
//! Named-query only — the tool never carries raw SQL, so the registry's
//! `invoke_named` (which validates the name and parameter arity) is the only
//! entry point exposed to the model (docs/design/datasources.md "AI"). This is
//! the lenient (read) path: a cap breach truncates and is reported via the
//! `breached` flag, the same as a dashboard read. The strict breach-is-an-error
//! policy is the board `datasource` node's, not the AI tool's.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_datasource::{DatasourceRegistry, Param};
use rubix_tools::DatasourceAccess;

/// Registry-backed datasource access handed to the datasource AI tools.
#[derive(Clone)]
pub struct RegistryDatasourceAccess {
    registry: Arc<DatasourceRegistry>,
}

impl RegistryDatasourceAccess {
    pub fn new(registry: Arc<DatasourceRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl DatasourceAccess for RegistryDatasourceAccess {
    async fn invoke_named(
        &self,
        datasource: &str,
        name: &str,
        params: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let params = params
            .into_iter()
            .map(|v| {
                serde_json::from_value::<Param>(v)
                    .map_err(|e| anyhow::anyhow!("invalid datasource parameter: {e}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let executor = self.registry.executor(datasource)?;
        // Lenient (read) path: a breach is reported via `breached`, not an error.
        let result = executor.invoke_named(name, &params).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn describe(&self, datasource: &str) -> anyhow::Result<serde_json::Value> {
        let schema = self.registry.describe(datasource).await?;
        Ok(serde_json::to_value(schema)?)
    }
}
