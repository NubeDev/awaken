//! Run one read-only query that spans SurrealDB and the registered connectors.
//!
//! This is how "the unified query surface reads from the registry"
//! (`rubix/docs/SCOPE.md`, "Datasources"): build the native SurrealDB context
//! through the caller's scoped session (`rubix-query`, so contract #1 still bounds
//! the SurrealDB rows), register every external connector's `TableProvider` on the
//! same context under its datasource id, then plan and run a single read-only
//! `SELECT`/`WITH` across all of them. The query action is itself app-enforced —
//! gated on the WS-04 `external-query` capability before any scan, fail closed
//! (contract #2). External tables are addressed schema-qualified as
//! `"<datasource id>"."<table>"`, so they never collide with the native canonical
//! table names.

use std::sync::Arc;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::SchemaProvider;
use datafusion::catalog::memory::MemorySchemaProvider;
use datafusion::datasource::TableProvider;
use datafusion::execution::SendableRecordBatchStream;
use rubix_gate::{Capability, ScopedSession, check_capability};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::{ContextCache, ScopeIdentity, build_context_cached, ensure_read_only};

use crate::error::{DatasourceError, Result};

use super::entry::DatasourceEntry;
use super::store::Registry;

/// The capability the spanning query action requires (same as the native query
/// surface, WS-09): exercising the DataFusion plane is `external-query`.
const QUERY_CAPABILITY: Capability = Capability::ExternalQuery;

/// Run `sql` across SurrealDB plus every registered connector, for the principal
/// of `session`.
///
/// `grant_reader` reads the `grant` table (the root handle's connection) for the
/// capability check; `session` is the principal's scoped read session — the
/// SurrealDB rows are bounded by its row-level permissions, and the external
/// tables are the connectors registered in `registry`.
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the query
///   capability check.
/// - [`DatasourceError::Query`] if the statement is not a single read-only query.
/// - [`DatasourceError::DataFusion`] if registration or execution fails.
pub async fn span(
    registry: &Registry,
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
    cache: &ContextCache,
    sql: &str,
) -> Result<Vec<RecordBatch>> {
    authorize_query(grant_reader, session).await?;
    let ctx = build_spanning_context(registry, session, cache).await?;
    run_one(&ctx, sql).await
}

/// Like [`span`] but return a **lazy result stream** instead of collecting every
/// batch — the streaming entry the Tier-2 query job pumps over the WS plane
/// (`rubix/docs/design/BULK-AND-JOBS.md`, "Streaming query").
///
/// Building the stream is cheap (authorize + build the context + plan); the scan
/// work happens as the caller pulls batches, so a wide timeseries read is never
/// materialised in full at the HTTP boundary. The returned
/// [`SendableRecordBatchStream`] owns its physical plan (via `Arc`), so it outlives
/// the borrowed `registry`/`session`/`cache` — a spawned job can pump it after the
/// request returns.
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the query
///   capability check.
/// - [`DatasourceError::Query`] if the statement is not a single read-only query.
/// - [`DatasourceError::DataFusion`] if registration or planning fails.
pub async fn span_stream(
    registry: &Registry,
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
    cache: &ContextCache,
    sql: &str,
) -> Result<SendableRecordBatchStream> {
    authorize_query(grant_reader, session).await?;
    let ctx = build_spanning_context(registry, session, cache).await?;
    ensure_read_only(sql)?;
    let dataframe = ctx.sql(sql).await?;
    let stream = dataframe.execute_stream().await?;
    Ok(stream)
}

/// Run several read-only statements for the principal against **one** built
/// context, returning a per-statement result so a single bad query never fails
/// the batch (`rubix/docs/design/DASHBOARDS-SCOPE.md` §3).
///
/// The capability check and the context build (the scoped scan of every canonical
/// table + external registration) happen **once**, then each statement is guarded
/// and planned on the shared context independently. A statement that fails to
/// guard, plan, or execute comes back as `Err(message)` in its slot; the others
/// still return their rows. The whole call only errors for a request-level
/// failure — a missing capability or a context build that fails — never for one
/// bad statement (batching is transport, not a permission shortcut: every
/// statement still runs through the same guard and the same scoped session).
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the query
///   capability check.
/// - [`DatasourceError::DataFusion`] if building the shared context fails.
pub async fn span_batch(
    registry: &Registry,
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
    cache: &ContextCache,
    statements: &[String],
) -> Result<Vec<std::result::Result<Vec<RecordBatch>, String>>> {
    authorize_query(grant_reader, session).await?;
    let ctx = build_spanning_context(registry, session, cache).await?;

    let mut results = Vec::with_capacity(statements.len());
    for sql in statements {
        results.push(run_one(&ctx, sql).await.map_err(|e| e.to_string()));
    }
    Ok(results)
}

/// Check the principal holds the query capability, fail closed otherwise.
pub(crate) async fn authorize_query(
    grant_reader: &Surreal<Db>,
    session: &ScopedSession,
) -> Result<()> {
    let granted = check_capability(grant_reader, session.principal(), QUERY_CAPABILITY)
        .await
        .map_err(|e| DatasourceError::Capability(e.to_string()))?;
    if !granted {
        return Err(DatasourceError::Denied);
    }
    Ok(())
}

/// Build the spanning DataFusion context: the scoped native scan (cached per
/// principal, §4a) plus every registered external connector's tables.
///
/// The native canonical scan is the expensive, cacheable part and is keyed on the
/// principal's identity so one principal's rows are never served to another. The
/// external connectors' providers are already materialised once in the registry,
/// so they are registered fresh on each context (cheap pointer registration) and
/// stay outside the per-principal cache.
pub(crate) async fn build_spanning_context(
    registry: &Registry,
    session: &ScopedSession,
    cache: &ContextCache,
) -> Result<datafusion::prelude::SessionContext> {
    let principal = session.principal();
    let scope = ScopeIdentity::new(principal.namespace.clone(), principal.subject.to_string());
    let ctx = build_context_cached(session.connection(), cache, &scope).await?;
    register_external_tables(&ctx, registry)?;
    Ok(ctx)
}

/// Guard, plan, and execute one read-only statement on `ctx`.
async fn run_one(ctx: &datafusion::prelude::SessionContext, sql: &str) -> Result<Vec<RecordBatch>> {
    ensure_read_only(sql)?;
    let dataframe = ctx.sql(sql).await?;
    let batches = dataframe.collect().await?;
    Ok(batches)
}

/// Register every external connector's providers on `ctx`, schema-qualified by id.
///
/// Each datasource id becomes a schema in the default catalog holding that
/// connector's tables, so a query addresses them as `"<id>"."<table>"` with no
/// collision against the native canonical tables (registered bare on `ctx`).
/// `register_table` does not create a missing schema, so the schema is registered
/// explicitly first.
fn register_external_tables(
    ctx: &datafusion::prelude::SessionContext,
    registry: &Registry,
) -> Result<()> {
    let default_catalog = ctx
        .copied_config()
        .options()
        .catalog
        .default_catalog
        .clone();
    let catalog = ctx.catalog(&default_catalog).ok_or_else(|| {
        DatasourceError::Query(format!("missing default catalog `{default_catalog}`"))
    })?;

    for (id, entry) in registry.entries() {
        let DatasourceEntry::External { tables, .. } = entry else {
            continue;
        };
        let schema = match catalog.schema(id) {
            Some(schema) => schema,
            None => {
                let schema: Arc<dyn SchemaProvider> = Arc::new(MemorySchemaProvider::new());
                catalog
                    .register_schema(id, Arc::clone(&schema))
                    .map_err(DatasourceError::DataFusion)?;
                schema
            }
        };
        for (table, provider) in tables {
            let provider: Arc<dyn TableProvider> = Arc::clone(provider);
            schema
                .register_table(table.clone(), provider)
                .map_err(DatasourceError::DataFusion)?;
        }
    }
    Ok(())
}
