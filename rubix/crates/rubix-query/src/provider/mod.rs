//! Expose the canonical SurrealDB tables to DataFusion, read-only.
//!
//! Each canonical table is scanned through the principal's scoped session (so
//! SurrealDB row-level permissions decide visibility, contract #1) into an Arrow
//! batch and registered as an in-memory table in a DataFusion `SessionContext`.
//! DataFusion then plans the read-only SQL over those tables — it sits **above**
//! SurrealDB for unification/aggregation, with SurrealQL doing the row read
//! (`rubix/STACK-DEISGN.md`, contract #6). The providers are registered under the
//! canonical table names only; there is no unscoped base table to escape the
//! principal's permissions, because the only rows present are the ones the scoped
//! scan already returned.

mod cache;
mod instant;
mod json_udf;
mod scan;
mod schema;

use std::sync::Arc;

use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;

pub use cache::{ContextCache, ScopeIdentity};
pub use schema::CanonicalTable;

pub(crate) use instant::parse_created_micros;
pub(crate) use scan::scan_table;

/// One canonical table scanned into a registrable in-memory provider.
pub(crate) type ScannedTable = (&'static str, Arc<MemTable>);

/// Scan every canonical table through `session` into registrable providers.
///
/// This is the expensive step the cache memoizes (§4a): each canonical table is
/// read through the principal's scoped session into an Arrow batch and wrapped in
/// a [`MemTable`]. The providers hold **raw canonical values** — no unit
/// conversion or formatting — so a cached scan is reusable across callers with
/// different display preferences (§2). Returned as `Arc`-wrapped providers so a
/// cache hit clones the pointer, not the data.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if any table scan or provider
/// construction fails.
pub(crate) async fn scan_all_tables(session: &Surreal<Db>) -> Result<Vec<ScannedTable>> {
    let mut tables = Vec::with_capacity(CanonicalTable::ALL.len());
    for table in CanonicalTable::ALL {
        let batch = scan_table(session, table).await?;
        let provider = MemTable::try_new(batch.schema(), vec![vec![batch]])
            .map_err(crate::QueryError::DataFusion)?;
        tables.push((table.register_name(), Arc::new(provider)));
    }
    Ok(tables)
}

/// Build a fresh DataFusion context registering `tables` plus the `json_get` UDF.
///
/// The context itself is cheap to build; the scan is what the cache holds. A
/// cache hit therefore still gets a fresh context (so per-request state never
/// bleeds across callers) but reuses the scanned providers.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if registering a provider fails.
pub(crate) fn context_from_tables(tables: &[ScannedTable]) -> Result<SessionContext> {
    let ctx = SessionContext::new();
    ctx.register_udf(json_udf::json_get_udf());
    for (name, provider) in tables {
        let provider = Arc::clone(provider) as Arc<dyn datafusion::datasource::TableProvider>;
        ctx.register_table(*name, provider)
            .map_err(crate::QueryError::DataFusion)?;
    }
    Ok(ctx)
}

/// Build a DataFusion context with every canonical table registered, scanned
/// through `session`.
///
/// The returned context holds only rows the scoped session was permitted to
/// read; planning and executing SQL against it cannot reach any other row. A
/// table no writer has populated registers as an empty table, so a query over it
/// returns no rows rather than failing to resolve the name.
///
/// This is the entry the pluggable-datasource layer (`rubix-datasource`, WS-10)
/// extends: it builds the native SurrealDB context here, then registers each
/// connector's `TableProvider` on the same context so a query spans SurrealDB and
/// the external sources (`rubix/docs/SCOPE.md`, "Datasources").
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if any table scan or registration
/// fails.
pub async fn build_context(session: &Surreal<Db>) -> Result<SessionContext> {
    let tables = scan_all_tables(session).await?;
    context_from_tables(&tables)
}

/// Build a context for `scope`, reusing a cached scan when one is live (§4a).
///
/// On a cache hit the canonical tables are **not** rescanned — the cached
/// providers (raw canonical values) are registered into a fresh context, so a
/// board tick and different SQL on the same tables avoid the dominant rescan
/// cost. On a miss the tables are scanned through `session` and the scan is cached
/// under `scope` before the context is returned. The scope identity keys the
/// cache so one principal's scan is never served to another (the cross-principal
/// leak §4a flags); the SQL still runs fresh on top of the returned context.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if a scan or registration fails.
pub async fn build_context_cached(
    session: &Surreal<Db>,
    cache: &ContextCache,
    scope: &ScopeIdentity,
) -> Result<SessionContext> {
    if let Some(tables) = cache.get(scope) {
        return context_from_tables(&tables);
    }
    let tables = scan_all_tables(session).await?;
    cache.put(scope.clone(), tables.clone());
    context_from_tables(&tables)
}
