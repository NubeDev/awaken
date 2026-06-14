//! The deserializable shape of a `datasources.json` entry: id, connection
//! components, caps, pool sizing, named queries, and an optional schema blob.

mod entry;
mod named;
mod schema;

pub use entry::{CapsSpec, ConnectionSpec, DatasourceEntry, PoolSpec};
pub use named::NamedQuery;
pub use schema::{ColumnSchema, SchemaBlob, TableSchema};
