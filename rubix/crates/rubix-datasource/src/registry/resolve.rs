//! Resolve a datasource id to the table providers registered under it.
//!
//! The lookup the unified query surface uses to find a registered connector's
//! tables (`rubix/docs/SCOPE.md`, "Datasources"). An unknown id fails closed with
//! [`DatasourceError::Unknown`] — a query naming a datasource no connector was
//! registered under is denied, never silently treated as empty. The native
//! SurrealDB entry resolves to no external providers: its tables are scanned per
//! query through the scoped session by `rubix-query`, not stored here.

use std::sync::Arc;

use datafusion::datasource::TableProvider;

use crate::error::{DatasourceError, Result};

use super::entry::DatasourceEntry;
use super::store::Registry;

/// The `(table name, provider)` pairs a registered datasource id offers.
///
/// # Errors
/// Returns [`DatasourceError::Unknown`] if no datasource is registered under `id`.
pub fn resolve<'a>(
    registry: &'a Registry,
    id: &str,
) -> Result<&'a [(String, Arc<dyn TableProvider>)]> {
    match registry.get(id) {
        Some(DatasourceEntry::External { tables, .. }) => Ok(tables.as_slice()),
        // The native entry exists but holds no stored providers (scanned per
        // query through the scoped session), so it resolves to an empty slice.
        Some(DatasourceEntry::Native { .. }) => Ok(&[]),
        None => Err(DatasourceError::Unknown(id.to_owned())),
    }
}
