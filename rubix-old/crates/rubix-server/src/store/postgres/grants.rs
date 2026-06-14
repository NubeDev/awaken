//! Grant rows, Postgres backend. Mirrors [`super::super::grants`].

use uuid::Uuid;

use super::super::codec::ts_of;
use super::super::grants::{GrantRecord, Permission, SubjectKind, GRANT_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{ts_col, uuid_of};

fn grant_of(row: &postgres::Row) -> Result<GrantRecord> {
    let kind_raw: String = row.get(2);
    let perm_raw: String = row.get(6);
    Ok(GrantRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        subject_kind: SubjectKind::parse(&kind_raw)
            .ok_or_else(|| StoreError::Db(anyhow::anyhow!("bad subject_kind `{kind_raw}`")))?,
        subject_id: row.get(3),
        resource_kind: row.get(4),
        resource_ref: row.get(5),
        permission: Permission::parse(&perm_raw)
            .ok_or_else(|| StoreError::Db(anyhow::anyhow!("bad permission `{perm_raw}`")))?,
        created_at: ts_col(row, 7)?,
    })
}

pub(crate) fn create_grant(store: &Store, grant: &GrantRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO grants (id, org, subject_kind, subject_id, resource_kind, \
         resource_ref, permission, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &grant.id.to_string(),
            &grant.org,
            &grant.subject_kind.as_str().to_string(),
            &grant.subject_id,
            &grant.resource_kind,
            &grant.resource_ref,
            &grant.permission.as_str().to_string(),
            &ts_of(&grant.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_grants(
    store: &Store,
    org: &str,
    resource_ref: Option<&str>,
) -> Result<Vec<GrantRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {GRANT_COLS} FROM grants \
         WHERE org = $1 AND ($2::TEXT IS NULL OR resource_ref = $2) \
         ORDER BY created_at DESC"
    );
    client
        .query(sql.as_str(), &[&org, &resource_ref])?
        .iter()
        .map(grant_of)
        .collect()
}

pub(crate) fn get_grant(store: &Store, id: Uuid) -> Result<GrantRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {GRANT_COLS} FROM grants WHERE id = $1");
    match client.query_opt(sql.as_str(), &[&id.to_string()])? {
        Some(row) => grant_of(&row),
        None => Err(StoreError::NotFound("grant")),
    }
}

pub(crate) fn delete_grant(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM grants WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("grant"));
    }
    Ok(())
}

pub(crate) fn grants_for_subjects(
    store: &Store,
    org: &str,
    subjects: &[(SubjectKind, String)],
) -> Result<Vec<GrantRecord>> {
    // (subject_kind, subject_id) IN-list via VALUES join. org is $1; each pair
    // adds two params. Only `$N` markers — no injection surface.
    let mut client = store.postgres_conn()?;
    let mut sql = format!("SELECT {GRANT_COLS} FROM grants WHERE org = $1 AND (");
    let mut params: Vec<Box<dyn postgres::types::ToSql + Sync>> = vec![Box::new(org.to_string())];
    for (i, (kind, id)) in subjects.iter().enumerate() {
        if i > 0 {
            sql.push_str(" OR ");
        }
        let k = 2 + i * 2;
        let v = k + 1;
        sql.push_str(&format!("(subject_kind = ${k} AND subject_id = ${v})"));
        params.push(Box::new(kind.as_str().to_string()));
        params.push(Box::new(id.clone()));
    }
    sql.push(')');
    let refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
        params.iter().map(|b| b.as_ref()).collect();
    client
        .query(sql.as_str(), &refs)?
        .iter()
        .map(grant_of)
        .collect()
}
