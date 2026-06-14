//! The native SurrealDB datasource as a `Connector`.
//!
//! SurrealDB is the native/default datasource (`rubix/docs/SCOPE.md`,
//! "Datasources"); the registry already seeds it as the default entry
//! (`super::registry::Registry::with_native_default`). This module additionally
//! lets a SurrealDB session be registered as a *second*, explicitly-scoped
//! datasource — the same engine, a different declared id — so a query can union
//! the caller's own scoped tables with another SurrealDB scope. It reuses
//! `rubix-query`'s scoped scan rather than re-reading rows itself
//! (`docs/FILE-LAYOUT.md`, dedup).

mod connect;

pub use connect::SurrealConnector;
