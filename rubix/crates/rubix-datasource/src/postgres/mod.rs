//! The Postgres datasource connector (cloud-only, feature-gated).
//!
//! Postgres attaches as a connector through `datafusion-table-providers`
//! (`rubix/docs/SCOPE.md`, "Datasources": Postgres is a pluggable source, not part
//! of the core). It is gated behind the `postgres` cargo feature so the edge build
//! carries no Postgres backend (`rubix/STACK-DEISGN.md`, "Edge is the default
//! build"); when the feature is absent this module does not compile in, so the
//! connector fails closed — there is no runtime path that silently degrades.

mod connect;

pub use connect::PostgresConnector;
