//! Build a `Connector` over a SurrealDB session's canonical tables.
//!
//! The native engine exposed through the same connector contract as any external
//! source (`rubix/docs/SCOPE.md`, "Datasources"). Materialising a provider scans
//! the canonical tables through the given session with `rubix-query`'s scoped scan
//! — so the rows are exactly what that session's SurrealDB row-level permissions
//! admit (contract #1) — and hands DataFusion the resulting in-memory provider.
//! The scan is reused, not reimplemented (`docs/FILE-LAYOUT.md`, dedup).

use std::sync::Arc;

use datafusion::datasource::TableProvider;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::{CanonicalTable, build_context};

use crate::connector::{Connector, DatasourceConfig};
use crate::error::{DatasourceError, Result};

/// A connector that exposes one SurrealDB session's canonical tables.
///
/// Holds the declared identity and the session whose scope the tables are read
/// under. Building a provider materialises that table through the scoped scan.
pub struct SurrealConnector {
    config: DatasourceConfig,
    session: Surreal<Db>,
}

impl SurrealConnector {
    /// Declare a SurrealDB datasource with `id`/`label` over `session`.
    ///
    /// `session` is a gate-issued scoped connection; the tables this connector
    /// offers are read under its permissions.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, session: Surreal<Db>) -> Self {
        Self {
            config: DatasourceConfig::new(id, label),
            session,
        }
    }
}

impl Connector for SurrealConnector {
    fn config(&self) -> &DatasourceConfig {
        &self.config
    }

    fn tables(&self) -> Vec<String> {
        CanonicalTable::ALL
            .into_iter()
            .map(|table| table.register_name().to_owned())
            .collect()
    }

    async fn table_provider(&self, table: &str) -> Result<Arc<dyn TableProvider>> {
        if CanonicalTable::parse(table).is_none() {
            return Err(DatasourceError::Connect {
                id: self.config.id().to_owned(),
                reason: format!("`{table}` is not a canonical SurrealDB table"),
            });
        }
        let ctx = build_context(&self.session).await?;
        ctx.table_provider(table)
            .await
            .map_err(DatasourceError::DataFusion)
    }
}
