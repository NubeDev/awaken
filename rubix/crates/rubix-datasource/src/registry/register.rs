//! Register a connector under its id, capability-checked and materialised.
//!
//! Adding a datasource is adding a registry entry (`rubix/docs/SCOPE.md`,
//! "Datasources"), gated by the WS-04 `datasource-register` capability (contract
//! #2). The pipeline: authorize the principal (fail closed) → refuse a duplicate
//! or the reserved native id → build each of the connector's table providers once
//! → store them under the connector's id. The providers are materialised here, at
//! registration, not per query, so the connection cost is paid up front and a
//! query just unions the already-built providers.

use rubix_core::Principal;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::connector::Connector;
use crate::error::{DatasourceError, Result};

use super::authorize::authorize_register;
use super::entry::DatasourceEntry;
use super::store::{NATIVE_SURREAL_ID, Registry};

/// Register `connector` into `registry` on behalf of `principal`.
///
/// `grant_reader` reads the `grant` table (the root handle's connection) for the
/// capability check. The connector's id must be unique and must not be the
/// reserved native SurrealDB id. Every table the connector offers is materialised
/// into a `TableProvider` and stored, so a later query spans them without
/// reconnecting.
///
/// # Errors
/// - [`DatasourceError::Denied`] / [`DatasourceError::Capability`] from the
///   capability check.
/// - [`DatasourceError::Duplicate`] if the id is already registered (including the
///   reserved native id).
/// - [`DatasourceError::Connect`] if a connector table fails to build its
///   provider.
pub async fn register<C: Connector>(
    registry: &mut Registry,
    grant_reader: &Surreal<Db>,
    principal: &Principal,
    connector: C,
) -> Result<()> {
    authorize_register(grant_reader, principal).await?;

    let id = connector.config().id().to_owned();
    if id == NATIVE_SURREAL_ID || registry.contains(&id) {
        return Err(DatasourceError::Duplicate(id));
    }

    let mut tables = Vec::new();
    for table in connector.tables() {
        let provider = connector
            .table_provider(&table)
            .await
            .map_err(|e| DatasourceError::Connect {
                id: id.clone(),
                reason: e.to_string(),
            })?;
        tables.push((table, provider));
    }

    registry.insert(
        id,
        DatasourceEntry::External {
            config: connector.config().clone(),
            tables,
        },
    );
    Ok(())
}
