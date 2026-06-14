//! Read-only access to external SQL databases (primarily TimescaleDB/Postgres)
//! by running operator-authored native SQL and returning rows.
//!
//! This is the core engine for the datasource model in
//! `rubix/docs/design/datasources.md`: a declared, read-only connection to an
//! external SQL database that runs native SQL — DataFusion is not in the path,
//! the external engine plans and executes. The returned `{ columns, rows }`
//! shape matches `rubix-query` so every consumer renders or folds it the same
//! way.
//!
//! ## Surface
//!
//! - [`DatasourceRegistry`] — owns credentials and one small pool per
//!   datasource id; the only component that sees a decrypted password. Callers
//!   pass only an id.
//! - [`Executor`] — borrowed from the registry per id; two entry points over
//!   one validation+caps path: [`Executor::execute`] (operator-authored raw
//!   SQL) and [`Executor::invoke_named`] (named-query invocation, the AI tier).
//! - Manifest types ([`DatasourceEntry`] and friends) — the deserializable
//!   `datasources.json` entry shape.
//! - [`Caps`]/[`CapState`] — row/byte/wall-clock bounds; breach is surfaced as
//!   an inspectable `breached` flag (lenient/dashboard path) or, via
//!   [`Executor::strict`], a [`DatasourceError::CapBreached`] (strict/spark
//!   path). The policy is the caller's, not baked in.
//! - [`SqlBackend`] — the thin SQL-execution seam; the sqlx [`PostgresBackend`]
//!   is the one implementation. Everything above it is unit-testable against a
//!   fake.
//!
//! ## Read-only by construction
//!
//! The crate never issues a write statement. The real guarantee is the
//! database role: a datasource connects as a `SELECT`-only user (docs
//! "Safety"). As defense in depth the backend also rejects multi-statement
//! input ([`statement`]) and sets `default_transaction_read_only`. These are
//! belt-and-braces; the read-only role is the primary mechanism.
//!
//! ## Integration
//!
//! None. This crate is the standalone core engine — it is not wired into
//! rubix-server, rubix-flow, rubix-query, the UI, or any running binary; no
//! `datasources.json` is loaded here. Another session owns integration.

mod backend;
mod caps;
mod describe;
mod error;
mod executor;
mod manifest;
mod registry;
mod statement;

pub use backend::{Column, PostgresBackend, PostgresConn, RawResult, ResultSet, Row, SqlBackend};
pub use caps::{CapState, Caps};
pub use describe::describe;
pub use error::{DatasourceError, DatasourceResult};
pub use executor::Executor;
pub use manifest::{
    CapsSpec, ColumnSchema, ConnectionSpec, DatasourceEntry, NamedQuery, PoolSpec, SchemaBlob,
    TableSchema,
};
pub use registry::DatasourceRegistry;
pub use statement::{ensure_single_statement, Param, Params};
