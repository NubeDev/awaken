//! What the registry stores under each datasource id.
//!
//! Two shapes, because the native SurrealDB datasource and external connectors are
//! materialised at different times (contract #1, `rubix/STACK-DEISGN.md`). The
//! native entry is scanned through the caller's scoped session per query, so it
//! carries no pre-built provider — only its declared identity. An external entry
//! built its providers once at registration and the registry reuses them.

use std::sync::Arc;

use datafusion::datasource::TableProvider;

use crate::connector::DatasourceConfig;

/// A datasource the registry knows about.
pub(crate) enum DatasourceEntry {
    /// The native SurrealDB datasource. Its tables are scanned per query through
    /// the caller's scoped session (`rubix-query`), so no provider is stored.
    Native { config: DatasourceConfig },
    /// An external connector whose providers were materialised at registration.
    External {
        config: DatasourceConfig,
        tables: Vec<(String, Arc<dyn TableProvider>)>,
    },
}

impl DatasourceEntry {
    /// The declared identity of this datasource.
    pub(crate) fn config(&self) -> &DatasourceConfig {
        match self {
            DatasourceEntry::Native { config } | DatasourceEntry::External { config, .. } => config,
        }
    }
}
