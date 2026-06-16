//! The durable `datasource` table: schema, row shape, and CRUD on the root handle.
//!
//! Server configuration, not tenant data (`rubix/docs/SCOPE.md`, "Datasources"):
//! the table carries no scoped-session permission and is read/written on the root
//! store handle, the same boundary the gate's `grant` table uses. A row is the
//! connector's declaration — enough to rebuild the live connector on restart. The
//! connection string is a deployment secret persisted here and never serialised
//! onto the wire (the transport DTO omits it).
//!
//! [`StoredDatasource`] is the in-memory declaration the control plane and the
//! registry pass around (its `id` is a plain string); [`DatasourceRow`] is its
//! persisted shape, whose `id` is the SurrealDB [`RecordId`] table key (mirroring
//! the gate's `GrantRow`) so the row key and the declared id stay one value.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, ToSql};

/// The table datasource declarations are stored in.
const DATASOURCE_TABLE: &str = "datasource";

/// The `datasource` config table, defined once at boot (idempotent).
///
/// `SCHEMALESS` to match the rest of the store's tables; the row shape is enforced
/// by [`DatasourceRow`] at the (de)serialise boundary, not by the engine.
const DATASOURCE_SCHEMA: &str = "DEFINE TABLE IF NOT EXISTS datasource SCHEMALESS;";

/// A persisted datasource declaration — everything needed to rebuild the connector.
///
/// Mirrors `rubix_datasource::DatasourceConfig` plus the connector wiring the
/// in-memory registry does not keep (`connection_string`, `tables`).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredDatasource {
    /// The stable id (the `datasource` table key and the registry key).
    pub id: String,
    /// The human-facing label.
    pub label: String,
    /// The connector family (`"postgres"`, …).
    pub kind: String,
    /// The connector's connection string — a secret, never returned on the wire.
    pub connection_string: String,
    /// The table names the connector exposes.
    pub tables: Vec<String>,
}

/// The SurrealDB-facing datasource row: the reserved `id` thing plus the fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct DatasourceRow {
    id: RecordId,
    label: String,
    kind: String,
    connection_string: String,
    tables: Vec<String>,
}

impl DatasourceRow {
    /// Project a declaration into its persisted row, keying `id` on the table.
    fn from_decl(decl: &StoredDatasource) -> Self {
        Self {
            id: RecordId::new(DATASOURCE_TABLE, decl.id.as_str()),
            label: decl.label.clone(),
            kind: decl.kind.clone(),
            connection_string: decl.connection_string.clone(),
            tables: decl.tables.clone(),
        }
    }

    /// Reconstruct the in-memory declaration, taking the id from the record key.
    fn into_decl(self) -> StoredDatasource {
        StoredDatasource {
            id: record_key(&self.id),
            label: self.label,
            kind: self.kind,
            connection_string: self.connection_string,
            tables: self.tables,
        }
    }
}

/// The raw string form of a datasource id's key (the part after `datasource:`).
fn record_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}

/// Define the `datasource` config table on the root handle. Idempotent.
///
/// # Errors
/// Returns the rendered SurrealDB error if the statement fails to apply.
pub async fn define_datasource_schema(db: &Surreal<Db>) -> Result<(), String> {
    db.query(DATASOURCE_SCHEMA)
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist (insert-or-replace) a datasource declaration by its id.
///
/// A content-replacing upsert keyed on the id, so re-saving (an update that
/// re-registers) overwrites the prior row rather than erroring on a duplicate key.
///
/// # Errors
/// Returns the rendered store error if the write fails.
pub async fn save(db: &Surreal<Db>, decl: &StoredDatasource) -> Result<(), String> {
    let _: Option<DatasourceRow> = db
        .upsert((DATASOURCE_TABLE, decl.id.as_str()))
        .content(DatasourceRow::from_decl(decl))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a datasource declaration by id. A missing row is a no-op (clean miss).
///
/// # Errors
/// Returns the rendered store error if the delete fails.
pub async fn forget(db: &Surreal<Db>, id: &str) -> Result<(), String> {
    let _: Option<DatasourceRow> = db
        .delete((DATASOURCE_TABLE, id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load every persisted datasource declaration, for boot rehydration.
///
/// # Errors
/// Returns the rendered store error if the table read fails.
pub async fn load_all(db: &Surreal<Db>) -> Result<Vec<StoredDatasource>, String> {
    let rows: Vec<DatasourceRow> = db
        .select(DATASOURCE_TABLE)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(DatasourceRow::into_decl).collect())
}
