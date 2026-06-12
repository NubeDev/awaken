//! The query engine: a `SessionContext` with the canonical tables registered.

mod open;
mod tables;

use std::sync::Arc;

use datafusion::prelude::SessionContext;

/// A DataFusion SQL surface over the rubix store.
///
/// Cheap to clone (the context is reference-counted). Built once per process
/// from the same SQLite database the HTTP store writes to.
#[derive(Clone)]
pub struct QueryEngine {
    ctx: Arc<SessionContext>,
}

impl QueryEngine {
    /// Borrow the underlying DataFusion context for advanced use (e.g.
    /// registering further providers or UDFs).
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }
}
