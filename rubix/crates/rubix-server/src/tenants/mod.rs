//! The tenant registry: the durable record of onboarded namespaces.
//!
//! A tenant is a **namespace**, not a domain entity (`rubix/docs/SCOPE.md`
//! non-goal: no tenant schema). Onboarding bootstraps a namespace and writes one
//! lightweight registry record so the namespace is discoverable
//! (`rubix/docs/design/ADMIN-API.md`, Surface 3). Like the `datasource` table,
//! the `tenant` table is server configuration, not tenant data: it carries no
//! scoped-session permission and is read/written on the root store handle, the
//! same boundary the gate's `grant` table uses. It is a *registry*, not a domain
//! schema — it carries no tenant *data*, so it does not violate the "no tenant
//! table" non-goal.

use chrono::{DateTime, Utc};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, ToSql};

/// The table tenant registry records are stored in.
const TENANT_TABLE: &str = "tenant";

/// The `tenant` registry table, defined once at boot (idempotent).
const TENANT_SCHEMA: &str = "DEFINE TABLE IF NOT EXISTS tenant SCHEMALESS;";

/// A registered tenant — the onboarding metadata for one namespace.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTenant {
    /// The tenant id (the namespace suffix and the registry key).
    pub id: String,
    /// The full namespace the tenant resolved to (`tenant_{id}`).
    pub namespace: String,
    /// When the tenant was onboarded.
    pub created_at: DateTime<Utc>,
    /// The full subject of the tenant's first admin (provisioned at onboarding).
    pub first_admin_subject: String,
}

/// The SurrealDB-facing tenant row: the reserved `id` thing plus the fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct TenantRow {
    id: RecordId,
    namespace: String,
    created_at: Datetime,
    first_admin_subject: String,
}

impl TenantRow {
    fn from_tenant(tenant: &StoredTenant) -> Self {
        Self {
            id: RecordId::new(TENANT_TABLE, tenant.id.as_str()),
            namespace: tenant.namespace.clone(),
            created_at: tenant.created_at.into(),
            first_admin_subject: tenant.first_admin_subject.clone(),
        }
    }

    fn into_tenant(self) -> StoredTenant {
        StoredTenant {
            id: record_key(&self.id),
            namespace: self.namespace,
            created_at: self.created_at.into(),
            first_admin_subject: self.first_admin_subject,
        }
    }
}

/// The raw string form of a tenant id's key (the part after `tenant:`).
fn record_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}

/// Define the `tenant` registry table on the root handle. Idempotent.
///
/// # Errors
/// Returns the rendered SurrealDB error if the statement fails to apply.
pub async fn define_tenant_schema(db: &Surreal<Db>) -> Result<(), String> {
    db.query(TENANT_SCHEMA)
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist a tenant registry record. Fails if the id already exists (onboarding
/// is non-idempotent — a tenant is created once).
///
/// # Errors
/// Returns the rendered store error if the write fails (including a duplicate id).
pub async fn create_tenant(db: &Surreal<Db>, tenant: &StoredTenant) -> Result<(), String> {
    let _: Option<TenantRow> = db
        .create((TENANT_TABLE, tenant.id.as_str()))
        .content(TenantRow::from_tenant(tenant))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Look up one tenant registry record by id.
///
/// # Errors
/// Returns the rendered store error if the read fails.
pub async fn get_tenant(db: &Surreal<Db>, id: &str) -> Result<Option<StoredTenant>, String> {
    let row: Option<TenantRow> = db
        .select((TENANT_TABLE, id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(TenantRow::into_tenant))
}

/// List every onboarded tenant registry record.
///
/// # Errors
/// Returns the rendered store error if the read fails.
pub async fn list_tenants(db: &Surreal<Db>) -> Result<Vec<StoredTenant>, String> {
    let rows: Vec<TenantRow> = db.select(TENANT_TABLE).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(TenantRow::into_tenant).collect())
}

/// Delete a tenant registry record by id. A missing row is a no-op.
///
/// # Errors
/// Returns the rendered store error if the delete fails.
pub async fn delete_tenant(db: &Surreal<Db>, id: &str) -> Result<(), String> {
    let _: Option<TenantRow> = db
        .delete((TENANT_TABLE, id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
