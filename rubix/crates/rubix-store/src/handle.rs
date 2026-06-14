//! The durable read/write boundary handle.
//!
//! Every later crate's persistence flows through this handle (contract: the
//! single store-and-brain engine, `rubix/STACK-DEISGN.md`). It owns the open,
//! bootstrapped SurrealDB connection and exposes the minimal durable read/write
//! surface; richer query verbs land in later workstreams that build on it.

use std::sync::Arc;

use rubix_core::RuntimeConfig;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::SurrealValue;

use crate::bootstrap::use_namespace;
use crate::check_health::probe;
use crate::connect::open_engine;
use crate::error::{Result, StoreError};
use crate::init_schema::init_schema;

/// A cloneable handle to the durable store.
///
/// Cloning is cheap (an `Arc` bump) so the handle can be shared across the
/// server's request handlers and later crate wiring.
#[derive(Clone)]
pub struct StoreHandle {
    db: Arc<Surreal<Db>>,
}

impl StoreHandle {
    /// Open the engine, select the namespace/database, and run schema init.
    ///
    /// This is the one entry point that produces a ready-to-use handle.
    ///
    /// # Errors
    /// Returns a [`StoreError`] if the engine cannot be opened, the
    /// namespace/database cannot be selected, or schema init fails.
    pub async fn open(config: &RuntimeConfig) -> Result<Self> {
        let db = open_engine(config).await?;
        use_namespace(&db, config).await?;
        init_schema(&db).await?;
        Ok(Self { db: Arc::new(db) })
    }

    /// Probe the underlying engine for liveness.
    ///
    /// # Errors
    /// Returns [`StoreError::Health`] if the engine does not answer.
    pub async fn health(&self) -> Result<()> {
        probe(&self.db).await
    }

    /// Borrow the raw client for verbs not yet wrapped by this handle.
    ///
    /// Later workstreams add typed read/write verbs here; until then this lets
    /// them build against the same connection without reopening the engine.
    #[must_use]
    pub fn raw(&self) -> &Surreal<Db> {
        &self.db
    }

    /// Create a single record at `(table, id)` with the given content.
    ///
    /// # Errors
    /// Returns [`StoreError::Operation`] if the write fails.
    pub async fn create<T>(&self, table: &str, id: &str, content: T) -> Result<Option<T>>
    where
        T: SurrealValue + 'static,
    {
        self.db
            .create((table.to_owned(), id.to_owned()))
            .content(content)
            .await
            .map_err(StoreError::Operation)
    }

    /// Read a single record at `(table, id)`.
    ///
    /// # Errors
    /// Returns [`StoreError::Operation`] if the read fails.
    pub async fn read<T>(&self, table: &str, id: &str) -> Result<Option<T>>
    where
        T: SurrealValue + 'static,
    {
        self.db
            .select((table.to_owned(), id.to_owned()))
            .await
            .map_err(StoreError::Operation)
    }
}
