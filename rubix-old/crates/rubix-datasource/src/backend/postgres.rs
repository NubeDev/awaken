//! The sqlx Postgres/TimescaleDB implementation of [`SqlBackend`].
//!
//! Pool setup mirrors nexus `federation/postgres_table.rs`: discrete creds via
//! `PgConnectOptions`, a small `max_connections` pool (the per-datasource
//! concurrency cap, docs "Per-datasource concurrency cap"). One pool is built
//! per datasource and owned by the registry; this type only borrows it.
//!
//! Reads use the `to_jsonb` path: `SELECT to_jsonb(t) FROM (<sql>) t` would
//! discard column order, so instead each row is read as a JSON object via
//! `row_to_json` over the user statement wrapped as a subquery, and the column
//! list comes from the first row's object keys. This keeps the crate Arrow-free
//! and free of a per-Postgres-type mapping table (prompt: "to_jsonb/Arrow-free
//! path — return serde_json rows, schema inferred from columns").
//!
//! Live-DB tests are `#[ignore]` and documented in `tests/`; this environment
//! has no Postgres, so the cap/param/single-statement/named-query logic is
//! tested against a fake backend instead.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Map, Value};
use sqlx_core::row::Row as _;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions};

use super::rows::{Column, Row};
use super::run::{RawResult, SqlBackend};
use crate::error::{DatasourceError, DatasourceResult};
use crate::statement::{Param, Params};

/// Discrete connection components for one Postgres/Timescale datasource. The
/// password is held only here and in the live pool; it is never logged and the
/// `Debug` impl redacts it (docs "Credentials").
#[derive(Clone)]
pub struct PostgresConn {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl std::fmt::Debug for PostgresConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresConn")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("user", &self.user)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// A sqlx pool to one datasource. Holds the datasource id only for error
/// messages; the id, not the creds, is what callers ever see.
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    datasource: String,
    pool: PgPool,
}

impl PostgresBackend {
    /// Open a small read-only pool to the datasource. `max_connections` is the
    /// per-datasource concurrency cap; `default_transaction_read_only` is set on
    /// every connection as defense in depth behind the read-only role.
    pub async fn connect(
        datasource: &str,
        conn: &PostgresConn,
        max_connections: u32,
    ) -> DatasourceResult<Self> {
        let opts = PgConnectOptions::new()
            .host(&conn.host)
            .port(conn.port)
            .database(&conn.database)
            .username(&conn.user)
            .password(&conn.password)
            // Defense in depth: the role is the real guarantee, but a read-only
            // transaction default rejects writes even if a grant is too broad.
            .options([("default_transaction_read_only", "on")]);
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_with(opts)
            .await
            .map_err(|e| DatasourceError::Connect {
                datasource: datasource.to_string(),
                message: e.to_string(),
            })?;
        Ok(Self {
            datasource: datasource.to_string(),
            pool,
        })
    }
}

/// Wrap a user statement so the engine plans it natively but each row comes back
/// as one JSON object preserving column names. `statement_timeout` is set as a
/// `LOCAL` GUC in the same simple call when a wall-clock cap is given; `LIMIT`
/// is applied as an outer bound when a fetch bound is given.
fn wrap_select(sql: &str, fetch_bound: Option<u64>) -> String {
    let limit = fetch_bound
        .map(|n| format!(" LIMIT {n}"))
        .unwrap_or_default();
    format!("SELECT row_to_json(__ds) AS __row FROM ({sql}) AS __ds{limit}")
}

/// Bind each typed [`Param`] positionally onto the query.
type PgQuery<'q> =
    sqlx_core::query::Query<'q, sqlx_postgres::Postgres, sqlx_postgres::PgArguments>;

fn bind<'q>(mut q: PgQuery<'q>, params: &'q Params) -> PgQuery<'q> {
    for p in params {
        q = match p {
            Param::Null => q.bind(Option::<String>::None),
            Param::Bool(b) => q.bind(*b),
            Param::Int(i) => q.bind(*i),
            Param::Float(f) => q.bind(*f),
            Param::Text(s) => q.bind(s.as_str()),
            // Bound as text; the engine casts against the column type. A future
            // backend may bind a native timestamp without changing the manifest.
            Param::Timestamp(s) => q.bind(s.as_str()),
        };
    }
    q
}

/// Turn JSON-object rows into ordered columns + ordered cell rows. Column order
/// is taken from the first row's key order (`row_to_json` preserves the select
/// list order). An empty result yields no columns — the describe path or a
/// caller that needs columns for an empty set should declare them in the
/// manifest (docs "Schema discovery").
fn shape(objects: Vec<Value>) -> RawResult {
    let mut columns: Vec<Column> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    if let Some(Value::Object(first)) = objects.first() {
        for k in first.keys() {
            keys.push(k.clone());
            columns.push(Column {
                name: k.clone(),
                // row_to_json erases SQL types; JSON value kind is all we know.
                type_name: "json".to_string(),
            });
        }
    }
    let rows: Vec<Row> = objects
        .into_iter()
        .map(|v| {
            let mut obj = match v {
                Value::Object(m) => m,
                other => {
                    let mut m = Map::new();
                    m.insert("__row".to_string(), other);
                    m
                }
            };
            keys.iter()
                .map(|k| obj.remove(k).unwrap_or(Value::Null))
                .collect()
        })
        .collect();
    RawResult { columns, rows }
}

#[async_trait]
impl SqlBackend for PostgresBackend {
    async fn run(
        &self,
        sql: &str,
        params: &Params,
        wall_clock: Option<Duration>,
        fetch_bound: Option<u64>,
    ) -> DatasourceResult<RawResult> {
        let mut tx = self.pool.begin().await.map_err(|e| self.backend_err(e))?;
        if let Some(d) = wall_clock {
            // Postgres native wall-clock cap. SET LOCAL is scoped to this tx and
            // takes no bind params, so the millis are formatted into a GUC name,
            // not into the user SQL.
            let ms = d.as_millis().max(1);
            sqlx_core::query::query(&format!("SET LOCAL statement_timeout = {ms}"))
                .execute(&mut *tx)
                .await
                .map_err(|e| self.backend_err(e))?;
        }
        let wrapped = wrap_select(sql, fetch_bound);
        let q = bind(sqlx_core::query::query(&wrapped), params);
        let pg_rows = q.fetch_all(&mut *tx).await.map_err(|e| self.backend_err(e))?;
        tx.commit().await.map_err(|e| self.backend_err(e))?;
        let objects: Vec<Value> = pg_rows
            .iter()
            .map(|r| r.try_get::<Value, _>("__row"))
            .collect::<Result<_, _>>()
            .map_err(|e| self.backend_err(e))?;
        Ok(shape(objects))
    }

    async fn introspect(&self) -> DatasourceResult<RawResult> {
        // Read column metadata under the read-only role. Restricted to the
        // user-facing schemas; system catalogs are excluded.
        let sql = "SELECT table_schema, table_name, column_name, data_type \
                   FROM information_schema.columns \
                   WHERE table_schema NOT IN ('pg_catalog','information_schema') \
                   ORDER BY table_schema, table_name, ordinal_position";
        self.run(sql, &Vec::new(), Some(Duration::from_secs(30)), None)
            .await
    }
}

impl PostgresBackend {
    fn backend_err(&self, e: sqlx_core::error::Error) -> DatasourceError {
        DatasourceError::Backend {
            datasource: self.datasource.clone(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_select_adds_limit_and_json_projection() {
        let w = wrap_select("SELECT 1", Some(10));
        assert!(w.contains("row_to_json"));
        assert!(w.ends_with(" LIMIT 10"));
        assert!(wrap_select("SELECT 1", None).contains("(SELECT 1)"));
    }

    #[test]
    fn shape_preserves_column_order_from_first_row() {
        let objects = vec![
            serde_json::json!({"b": 2, "a": 1}),
            serde_json::json!({"a": 9, "b": 8}),
        ];
        let r = shape(objects);
        assert_eq!(r.columns.iter().map(|c| &c.name).collect::<Vec<_>>(), ["b", "a"]);
        // Second row is reordered to match the first row's column order.
        assert_eq!(r.rows[1], vec![serde_json::json!(8), serde_json::json!(9)]);
    }

    #[test]
    fn shape_fills_missing_keys_with_null() {
        let objects = vec![
            serde_json::json!({"a": 1, "b": 2}),
            serde_json::json!({"a": 3}),
        ];
        let r = shape(objects);
        assert_eq!(r.rows[1], vec![serde_json::json!(3), Value::Null]);
    }

    #[test]
    fn debug_redacts_password() {
        let conn = PostgresConn {
            host: "h".into(),
            port: 5432,
            database: "d".into(),
            user: "u".into(),
            password: "supersecret".into(),
        };
        let s = format!("{conn:?}");
        assert!(!s.contains("supersecret"));
        assert!(s.contains("<redacted>"));
    }
}
