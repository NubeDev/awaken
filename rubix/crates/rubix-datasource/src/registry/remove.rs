//! Deregister a connector by id, capability-checked and native-guarded.
//!
//! Removing a datasource is removing its registry entry — the inverse of
//! [`register`](super::register::register) (`rubix/docs/SCOPE.md`, "Datasources"),
//! gated by the same WS-04 `datasource-register` capability (contract #2). The
//! pipeline: authorize the principal (fail closed) → refuse the reserved native id
//! → drop the entry, freeing its materialised providers. A query naming the id
//! afterwards fails closed through [`resolve`](super::resolve::resolve).

use rubix_core::Principal;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{DatasourceError, Result};

use super::authorize::authorize_register;
use super::store::{NATIVE_SURREAL_ID, Registry};

/// Deregister the datasource under `id` from `registry` on behalf of `principal`.
///
/// `grant_reader` reads the `grant` table (the root handle's connection) for the
/// capability check. The reserved native SurrealDB id cannot be removed.
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the
///   capability check.
/// - [`DatasourceError::Duplicate`] is *not* raised here; an absent id raises
///   [`DatasourceError::Unknown`] so a delete of a non-existent datasource is a
///   clean miss, not a silent success.
pub async fn remove(
    registry: &mut Registry,
    grant_reader: &Surreal<Db>,
    principal: &Principal,
    id: &str,
) -> Result<()> {
    authorize_register(grant_reader, principal).await?;

    if id == NATIVE_SURREAL_ID {
        // The native SurrealDB datasource is the always-present default; refusing
        // its removal keeps the query surface from losing its base scan.
        return Err(DatasourceError::Denied);
    }
    if registry.remove(id) {
        Ok(())
    } else {
        Err(DatasourceError::Unknown(id.to_owned()))
    }
}
