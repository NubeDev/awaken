//! The datasource registry: id -> the connector's materialised tables.
//!
//! The unified query surface reads from this registry (`rubix/docs/SCOPE.md`,
//! "Datasources"): SurrealDB is the native/default entry, and every other
//! connector is added under a stable id. Adding a datasource is adding a registry
//! entry, never changing the core. Registration is an app-enforced capability
//! (`authorize`, WS-04 `datasource-register`); lookup is `resolve`; the spanning
//! query that unions SurrealDB with the registered providers is `span`.
//!
//! The native SurrealDB entry holds no pre-built provider: its tables are scanned
//! per query through the *caller's* scoped session (contract #1), so they are
//! materialised by `rubix-query` at query time, not stored here. External
//! connectors (Postgres) build their providers once at registration and the
//! registry reuses them.

mod authorize;
mod entry;
mod find;
mod list;
mod register;
mod remove;
mod resolve;
mod span;
mod store;

pub use authorize::authorize_register;
pub use find::find;
pub use list::list;
pub use register::{register, register_materialized};
pub use remove::remove;
pub use resolve::resolve;
pub use span::span;
pub use store::{NATIVE_SURREAL_ID, Registry};
