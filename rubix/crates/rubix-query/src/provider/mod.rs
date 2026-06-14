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

mod scan;
mod schema;

use std::sync::Arc;

use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;

pub use schema::CanonicalTable;

pub(crate) use scan::scan_table;

/// Build a DataFusion context with every canonical table registered, scanned
/// through `session`.
///
/// The returned context holds only rows the scoped session was permitted to
/// read; planning and executing SQL against it cannot reach any other row. A
/// table no writer has populated registers as an empty table, so a query over it
/// returns no rows rather than failing to resolve the name.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if any table scan or registration
/// fails.
pub(crate) async fn build_context(session: &Surreal<Db>) -> Result<SessionContext> {
    let ctx = SessionContext::new();
    for table in CanonicalTable::ALL {
        let batch = scan_table(session, table).await?;
        let provider = MemTable::try_new(batch.schema(), vec![vec![batch]])
            .map_err(|e| crate::QueryError::DataFusion(e.into()))?;
        ctx.register_table(table.register_name(), Arc::new(provider))
            .map_err(crate::QueryError::DataFusion)?;
    }
    Ok(ctx)
}
