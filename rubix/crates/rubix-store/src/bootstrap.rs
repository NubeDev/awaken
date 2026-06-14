//! Select (and implicitly create) the namespace and database on an open engine.
//!
//! Edge resolves to a single namespace; cloud is namespace-per-tenant. This step
//! only points the connection at the configured namespace/database; tenant
//! resolution lives in the gate (`rubix/docs/SCOPE.md`, "Edge and cloud
//! profiles").

use rubix_core::RuntimeConfig;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Result, StoreError};

/// Point `db` at the namespace and database named in `config`.
///
/// # Errors
/// Returns [`StoreError::Bootstrap`] if the namespace/database cannot be used.
pub async fn use_namespace(db: &Surreal<Db>, config: &RuntimeConfig) -> Result<()> {
    db.use_ns(config.namespace.clone())
        .use_db(config.database.clone())
        .await
        .map_err(StoreError::Bootstrap)?;
    Ok(())
}
