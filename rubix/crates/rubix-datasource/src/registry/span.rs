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
use rubix_gate::{Capability, ScopedSession, check_capability};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::{build_context, ensure_read_only};

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
    sql: &str,
) -> Result<Vec<RecordBatch>> {
    let granted = check_capability(grant_reader, session.principal(), QUERY_CAPABILITY)
        .await
        .map_err(|e| DatasourceError::Capability(e.to_string()))?;
    if !granted {
        return Err(DatasourceError::Denied);
    }

    ensure_read_only(sql)?;
    let ctx = build_context(session.connection()).await?;
    register_external_tables(&ctx, registry)?;

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
    let default_catalog = ctx.copied_config().options().catalog.default_catalog.clone();
    let catalog = ctx
        .catalog(&default_catalog)
        .ok_or_else(|| DatasourceError::Query(format!("missing default catalog `{default_catalog}`")))?;

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
