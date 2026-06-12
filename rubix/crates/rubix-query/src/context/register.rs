//! Build a `SessionContext` with the canonical tables registered live.

use std::sync::Arc;

use datafusion::prelude::SessionContext;

use super::tables::CANONICAL;
use super::QueryEngine;
use crate::error::QueryError;
use crate::provider::SqliteTable;

impl QueryEngine {
    /// Build a fresh context with each canonical table registered under its
    /// bare name (so `SELECT * FROM points` resolves directly). Schema is read
    /// from SQLite at call time, so empty tables still expose their columns.
    pub(crate) fn session(&self) -> Result<SessionContext, QueryError> {
        let ctx = SessionContext::new();
        for &table in CANONICAL {
            let provider = SqliteTable::try_new(self.pool.clone(), table)?;
            ctx.register_table(table, Arc::new(provider))
                .map_err(|source| QueryError::Register { table, source })?;
        }
        Ok(ctx)
    }
}
