//! List the declared datasources, native default included.
//!
//! The Grafana model surfaces the set of declared datasources so a dashboard can
//! pick one (`rubix/docs/SCOPE.md`, "Datasources"). This verb returns each
//! registered datasource's declared identity ([`DatasourceConfig`]), the native
//! SurrealDB default and every added connector alike. It is a read over the
//! registry — no capability gate, since listing the names a principal could query
//! reveals nothing the query gate does not already govern.

use crate::connector::DatasourceConfig;

use super::store::Registry;

/// Every declared datasource's identity, in unspecified order.
#[must_use]
pub fn list(registry: &Registry) -> Vec<&DatasourceConfig> {
    registry.entries().map(|(_, entry)| entry.config()).collect()
}
