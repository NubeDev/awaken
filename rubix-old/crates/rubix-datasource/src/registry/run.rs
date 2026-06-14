//! The datasource registry: owns credentials and one pool per datasource id.
//!
//! Callers pass only a datasource id; the registry is the only component that
//! ever sees a decrypted password, holds it in the pool, and never logs it
//! (docs "Credentials"). Connection material arrives already-resolved in the
//! manifest entries — the crate does not read a secret store. Each datasource
//! gets one small pool (the per-datasource concurrency cap).
//!
//! The registry is fixed to the sqlx [`PostgresBackend`] because the executor
//! and describe layers are generic over [`SqlBackend`]; a future engine would
//! add a backend variant here without touching those layers.

use std::collections::HashMap;

use crate::backend::PostgresBackend;
use crate::caps::Caps;
use crate::describe::describe;
use crate::error::{DatasourceError, DatasourceResult};
use crate::executor::Executor;
use crate::manifest::{DatasourceEntry, NamedQuery, SchemaBlob};

/// One resolved datasource: its live pool, caps, named queries, and declared
/// schema. Credentials live inside `backend`'s pool and nowhere else.
struct Resolved {
    backend: PostgresBackend,
    caps: Caps,
    named: Vec<NamedQuery>,
    declared_schema: Option<SchemaBlob>,
}

/// Owns every datasource's pool and metadata, keyed by id.
#[derive(Default)]
pub struct DatasourceRegistry {
    sources: HashMap<String, Resolved>,
}

impl DatasourceRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve every manifest entry into a live pool and register it. Builds
    /// pools eagerly so a bad credential or unreachable host fails here, at
    /// registration, rather than on first read. The decrypted password is
    /// consumed into the pool and not retained anywhere else.
    pub async fn register_all(
        &mut self,
        entries: Vec<DatasourceEntry>,
    ) -> DatasourceResult<()> {
        for entry in entries {
            let backend = PostgresBackend::connect(
                &entry.id,
                &entry.connection.to_conn(),
                entry.pool.max_connections,
            )
            .await?;
            self.sources.insert(
                entry.id.clone(),
                Resolved {
                    backend,
                    caps: entry.caps.to_caps(),
                    named: entry.named_queries,
                    declared_schema: entry.schema,
                },
            );
        }
        Ok(())
    }

    /// Borrow an [`Executor`] for a datasource id, or error if unknown. The
    /// caller never sees the credentials behind it.
    pub fn executor(&self, id: &str) -> DatasourceResult<Executor<'_, PostgresBackend>> {
        let (key, r) = self
            .sources
            .get_key_value(id)
            .ok_or_else(|| DatasourceError::UnknownDatasource(id.to_string()))?;
        Ok(Executor {
            datasource: key.as_str(),
            backend: &r.backend,
            caps: r.caps,
            named: &r.named,
        })
    }

    /// Describe a datasource's schema: the operator-declared blob if present,
    /// else live introspection (docs "Schema discovery").
    pub async fn describe(&self, id: &str) -> DatasourceResult<SchemaBlob> {
        let r = self.resolved(id)?;
        describe(&r.backend, r.declared_schema.as_ref()).await
    }

    /// True if a datasource is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.sources.contains_key(id)
    }

    /// Every registered datasource id, sorted. Backs the board editor's
    /// `datasources` option source — the caller never sees credentials, only ids.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sources.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// The operator-registered named-query names for a datasource, sorted.
    /// Empty for an unknown id (the editor simply shows no choices). Backs the
    /// `datasource_named` option source.
    pub fn named_query_names(&self, id: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .sources
            .get(id)
            .map(|r| r.named.iter().map(|q| q.name.clone()).collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    fn resolved(&self, id: &str) -> DatasourceResult<&Resolved> {
        self.sources
            .get(id)
            .ok_or_else(|| DatasourceError::UnknownDatasource(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_datasource_errors() {
        let reg = DatasourceRegistry::new();
        assert!(!reg.contains("nope"));
        assert!(matches!(
            reg.executor("nope").err(),
            Some(DatasourceError::UnknownDatasource(_))
        ));
        let err = reg.describe("nope").await.unwrap_err();
        assert!(matches!(err, DatasourceError::UnknownDatasource(_)));
    }
}
