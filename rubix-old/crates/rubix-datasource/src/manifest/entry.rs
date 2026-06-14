//! The in-memory representation of one `datasources.json` entry.
//!
//! Minimal proposed JSON schema (the prompt invites a schema; kept minimal):
//! ```json
//! {
//!   "id": "site-historian",
//!   "connection": {
//!     "host": "db.example.com", "port": 5432,
//!     "database": "history", "user": "rubix_ro", "password": "..."
//!   },
//!   "caps": { "max_rows": 50000, "max_bytes": 8388608, "max_duration_ms": 15000 },
//!   "pool": { "max_connections": 4 },
//!   "named_queries": [
//!     { "name": "site_daily", "sql": "SELECT time_bucket('1 day', ts) ...",
//!       "param_count": 2 }
//!   ],
//!   "schema": { "tables": [ { "name": "readings",
//!     "columns": [ { "name": "ts", "type_name": "timestamptz" } ] } ] }
//! }
//! ```
//! `password` is read once into memory and handed to the pool; it is owned only
//! by the registry and never logged (docs "Credentials").

use serde::Deserialize;

use super::named::NamedQuery;
use super::schema::SchemaBlob;
use crate::backend::PostgresConn;
use crate::caps::Caps;

/// One declared datasource. Deserialized from a `datasources.json` entry; the
/// connection material is expected already-resolved (decrypted upstream — the
/// crate does not read a secret store, docs "Credentials").
#[derive(Debug, Clone, Deserialize)]
pub struct DatasourceEntry {
    /// Stable id callers reference. The only thing a caller ever passes.
    pub id: String,
    /// Discrete connection components (host/port/db/user/password, not a URI).
    pub connection: ConnectionSpec,
    /// Result caps applied to every read on this datasource.
    pub caps: CapsSpec,
    /// Pool sizing; the per-datasource concurrency cap.
    #[serde(default)]
    pub pool: PoolSpec,
    /// Operator-registered named queries the AI tier may invoke by name.
    #[serde(default)]
    pub named_queries: Vec<NamedQuery>,
    /// Optional operator-declared schema blob, returned by describe when present
    /// instead of (or alongside) live introspection (docs "Schema discovery").
    #[serde(default)]
    pub schema: Option<SchemaBlob>,
}

/// Discrete connection components. Mirrors nexus `PostgresConn`.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionSpec {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl ConnectionSpec {
    /// Convert to the backend's connection type. Consumes nothing the caller
    /// keeps; the registry calls this once at pool-build time.
    pub fn to_conn(&self) -> PostgresConn {
        PostgresConn {
            host: self.host.clone(),
            port: self.port,
            database: self.database.clone(),
            user: self.user.clone(),
            password: self.password.clone(),
        }
    }
}

/// Caps as declared in JSON (`max_duration_ms` rather than a Duration).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CapsSpec {
    pub max_rows: Option<u64>,
    pub max_bytes: Option<u64>,
    pub max_duration_ms: Option<u64>,
}

impl CapsSpec {
    /// Build the runtime [`Caps`] from the declared spec.
    pub fn to_caps(&self) -> Caps {
        Caps {
            max_rows: self.max_rows,
            max_bytes: self.max_bytes,
            max_duration: self
                .max_duration_ms
                .map(std::time::Duration::from_millis),
        }
    }
}

/// Pool sizing. Defaults small (the docs call for a small per-datasource pool).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PoolSpec {
    pub max_connections: u32,
}

impl Default for PoolSpec {
    fn default() -> Self {
        Self { max_connections: 4 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_entry() {
        let json = r#"{
            "id": "h",
            "connection": {"host":"db","port":5432,"database":"d","user":"ro","password":"p"},
            "caps": {"max_rows": 100, "max_bytes": 2048, "max_duration_ms": 5000}
        }"#;
        let e: DatasourceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.id, "h");
        assert_eq!(e.pool.max_connections, 4, "pool defaults small");
        assert!(e.named_queries.is_empty());
        assert!(e.schema.is_none());
        let caps = e.caps.to_caps();
        assert_eq!(caps.max_rows, Some(100));
        assert_eq!(caps.max_duration, Some(std::time::Duration::from_millis(5000)));
    }

    #[test]
    fn caps_spec_allows_open_axes() {
        let spec = CapsSpec {
            max_rows: Some(10),
            max_bytes: None,
            max_duration_ms: None,
        };
        let caps = spec.to_caps();
        assert_eq!(caps.max_bytes, None);
        assert_eq!(caps.max_duration, None);
    }
}
