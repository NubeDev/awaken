//! The `Connector` contract: a declared datasource that yields a `TableProvider`.
//!
//! The Grafana datasource model (`rubix/docs/SCOPE.md`, "Datasources"): a
//! datasource is a declared, pluggable connection a query can read in a unified
//! way, and each registers as a DataFusion `TableProvider`. "Unlimited
//! datasources" means adding a `Connector` impl, not changing the core. Every
//! connector carries its own configuration and exposes two things: the table
//! names it offers and, for each, the provider that scans it. SurrealDB is the
//! native/default connector (`super::surreal`); Postgres is a feature-gated one
//! (`super::postgres`).

mod config;

use std::sync::Arc;

use datafusion::datasource::TableProvider;

pub use config::{DatasourceConfig, NATIVE_KIND};

use crate::error::Result;

/// A pluggable datasource: declared config in, DataFusion `TableProvider` out.
///
/// A connector is identified by a stable [`DatasourceConfig::id`] and offers one
/// or more named tables. The unified query surface registers each offered table's
/// provider into the DataFusion context so a single `SELECT` can span SurrealDB
/// and any registered connector. Building a provider may touch the network (a
/// Postgres pool, say), so [`Connector::table_provider`] is async; it is called
/// once at registration, not per query, so the cost is paid up front.
///
/// The trait is consumed at registration time (its providers are materialised and
/// stored), so it never needs to be held as a trait object — keeping it free of
/// dyn-compatibility constraints from its async method.
pub trait Connector {
    /// The stable configuration this connector was declared with.
    fn config(&self) -> &DatasourceConfig;

    /// The table names this connector offers to the unified query surface.
    ///
    /// Each becomes a registrable DataFusion table; the surface plans `SELECT`s
    /// against these names alongside the native SurrealDB tables.
    fn tables(&self) -> Vec<String>;

    /// Build the DataFusion `TableProvider` for one of the connector's `tables`.
    ///
    /// # Errors
    /// Returns [`DatasourceError::Connect`](crate::DatasourceError::Connect) if the
    /// underlying source cannot be reached or the table does not exist there.
    fn table_provider(
        &self,
        table: &str,
    ) -> impl std::future::Future<Output = Result<Arc<dyn TableProvider>>> + Send;
}
