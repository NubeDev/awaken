//! Pluggable datasource connectors for the rubix query surface.
//!
//! The Grafana datasource model (`rubix/docs/SCOPE.md`, "Datasources"): a
//! datasource is a declared, pluggable connection a query can read in a unified
//! way, and each registers as a DataFusion `TableProvider`. "Unlimited
//! datasources" means adding a [`Connector`] impl, not changing the core. This
//! crate is the layer above `rubix-query`'s unified surface:
//!
//! - a [`Connector`] trait — declared config in, DataFusion `TableProvider` out;
//! - a [`Registry`] keyed by datasource id, seeded with the native SurrealDB
//!   default, that the spanning query reads from;
//! - [`register`] gated by the WS-04 `datasource-register` capability (contract
//!   #2, app-enforced) and [`span`] gated by `external-query`, both fail closed;
//! - a native [`SurrealConnector`] and a feature-gated Postgres connector
//!   (`#[cfg(feature = "postgres")]`), which is absent — and so fails closed — on
//!   the default edge build.

mod connector;
mod error;
mod registry;
mod surreal;

#[cfg(feature = "postgres")]
mod postgres;

pub use connector::{Connector, DatasourceConfig, NATIVE_KIND};
pub use error::{DatasourceError, Result};
pub use registry::{
    NATIVE_SURREAL_ID, Registry, authorize_register, find, list, register, register_materialized,
    remove, resolve, span, span_batch,
};
pub use surreal::SurrealConnector;

#[cfg(feature = "postgres")]
pub use postgres::PostgresConnector;
