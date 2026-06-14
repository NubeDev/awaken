//! Open the embedded SurrealDB engine selected by [`RuntimeConfig`].
//!
//! Contract #6 (`rubix/STACK-DEISGN.md`): SurrealDB is the single engine. The
//! only choice here is the embedded backend — in-memory for tests, file-backed
//! SurrealKV for a running node.

use rubix_core::{RuntimeConfig, StoreEngine};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, SurrealKv};

use crate::error::{Result, StoreError};

/// Open the engine described by `config`, returning the raw SurrealDB client.
///
/// The namespace/database are not selected here — that is the bootstrap step.
///
/// # Errors
/// Returns [`StoreError::Connect`] if the engine cannot be opened.
pub async fn open_engine(config: &RuntimeConfig) -> Result<Surreal<Db>> {
    match &config.engine {
        StoreEngine::Memory => Surreal::new::<Mem>(()).await.map_err(StoreError::Connect),
        StoreEngine::File { path } => Surreal::new::<SurrealKv>(path.as_str())
            .await
            .map_err(StoreError::Connect),
    }
}
