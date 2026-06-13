//! RBAC grant rows (Layer-2 ACL): a permission pinned on a resource (or `*`) for
//! a subject (a user or a team). Grants ADD access; they never subtract. The
//! two-layer authorization check reads them via [`Store::grants_for_subjects`].
//! Backend dispatch; SQLite body inline, Postgres in [`super::postgres::grants`].
//! See `docs/design/authz-rbac.md`.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{ts_of, ts_to};
use super::{Result, Store, StoreError};

/// Whether a grant's subject is a user or a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    User,
    Team,
}

impl SubjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SubjectKind::User => "user",
            SubjectKind::Team => "team",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(SubjectKind::User),
            "team" => Some(SubjectKind::Team),
            _ => None,
        }
    }
}

/// The permission a grant confers. Ordered: `Admin` ⊇ `Write` ⊇ `Read`, so a
/// higher grant satisfies a lower required action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Write,
    Admin,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Admin => "admin",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Permission::Read),
            "write" => Some(Permission::Write),
            "admin" => Some(Permission::Admin),
            _ => None,
        }
    }
    /// True when holding this permission satisfies a request for `required`.
    pub fn satisfies(self, required: Permission) -> bool {
        self >= required
    }
}

impl FromStr for Permission {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        Permission::parse(s).ok_or(())
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One `grants` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct GrantRecord {
    pub id: Uuid,
    pub org: String,
    #[schema(value_type = String)]
    pub subject_kind: SubjectKind,
    /// `user:<id>` / `team:<id>` target id (a UUID string).
    pub subject_id: String,
    /// The resource kind (`dashboard`, `board`, `rule`).
    pub resource_kind: String,
    /// The textual resource address, or `*` for all-of-kind within `org`.
    pub resource_ref: String,
    #[schema(value_type = String)]
    pub permission: Permission,
    pub created_at: DateTime<Utc>,
}

pub(crate) const GRANT_COLS: &str =
    "id, org, subject_kind, subject_id, resource_kind, resource_ref, permission, created_at";

fn row_grant(row: &Row<'_>) -> rusqlite::Result<GrantRecord> {
    let kind_raw: String = row.get(2)?;
    let perm_raw: String = row.get(6)?;
    Ok(GrantRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        subject_kind: SubjectKind::parse(&kind_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                format!("bad subject_kind `{kind_raw}`").into(),
            )
        })?,
        subject_id: row.get(3)?,
        resource_kind: row.get(4)?,
        resource_ref: row.get(5)?,
        permission: Permission::parse(&perm_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                format!("bad permission `{perm_raw}`").into(),
            )
        })?,
        created_at: ts_to(&row.get::<_, String>(7)?)?,
    })
}

impl Store {
    pub fn create_grant(&self, grant: &GrantRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_grant_sqlite(grant),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::grants::create_grant(self, grant),
        }
    }

    fn create_grant_sqlite(&self, grant: &GrantRecord) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO grants (id, org, subject_kind, subject_id, resource_kind, \
             resource_ref, permission, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                grant.id,
                grant.org,
                grant.subject_kind.as_str(),
                grant.subject_id,
                grant.resource_kind,
                grant.resource_ref,
                grant.permission.as_str(),
                ts_of(&grant.created_at),
            ],
        )?;
        Ok(())
    }

    /// Grants for an org, newest first. When `resource_ref` is `Some`, filters to
    /// grants on that exact resource (the per-dashboard grants view).
    pub fn list_grants(&self, org: &str, resource_ref: Option<&str>) -> Result<Vec<GrantRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_grants_sqlite(org, resource_ref),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::grants::list_grants(self, org, resource_ref),
        }
    }

    fn list_grants_sqlite(&self, org: &str, resource_ref: Option<&str>) -> Result<Vec<GrantRecord>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {GRANT_COLS} FROM grants \
             WHERE org = ?1 AND (?2 IS NULL OR resource_ref = ?2) \
             ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![org, resource_ref], row_grant)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_grant(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_grant_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::grants::delete_grant(self, id),
        }
    }

    fn delete_grant_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM grants WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("grant"));
        }
        Ok(())
    }

    /// Every grant in `org` held by any of `subjects` (`(kind, id)` pairs — the
    /// principal's user plus its teams). The Layer-2 input: read once per request
    /// and matched in memory against the resource. Empty `subjects` short-circuits.
    pub fn grants_for_subjects(
        &self,
        org: &str,
        subjects: &[(SubjectKind, String)],
    ) -> Result<Vec<GrantRecord>> {
        if subjects.is_empty() {
            return Ok(Vec::new());
        }
        match &self.backend {
            Backend::Sqlite(_) => self.grants_for_subjects_sqlite(org, subjects),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::grants::grants_for_subjects(self, org, subjects),
        }
    }

    fn grants_for_subjects_sqlite(
        &self,
        org: &str,
        subjects: &[(SubjectKind, String)],
    ) -> Result<Vec<GrantRecord>> {
        // Build a (subject_kind, subject_id) IN-list. Each pair is two bound
        // params; org is the first. No SQL injection surface — only `?` markers.
        let conn = self.sqlite_conn()?;
        let mut sql = format!("SELECT {GRANT_COLS} FROM grants WHERE org = ?1 AND (");
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(org.to_string())];
        for (i, (kind, id)) in subjects.iter().enumerate() {
            if i > 0 {
                sql.push_str(" OR ");
            }
            let k = 2 + i * 2;
            let v = k + 1;
            sql.push_str(&format!("(subject_kind = ?{k} AND subject_id = ?{v})"));
            binds.push(Box::new(kind.as_str().to_string()));
            binds.push(Box::new(id.clone()));
        }
        sql.push(')');
        let mut stmt = conn.prepare(&sql)?;
        let params = rusqlite::params_from_iter(binds.iter().map(|b| b.as_ref()));
        let rows = stmt.query_map(params, row_grant)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_grant(&self, id: Uuid) -> Result<GrantRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self
                .sqlite_conn()?
                .query_row(
                    &format!("SELECT {GRANT_COLS} FROM grants WHERE id = ?1"),
                    params![id],
                    row_grant,
                )
                .optional()?
                .ok_or(StoreError::NotFound("grant")),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::grants::get_grant(self, id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_ordering_satisfies_lower_actions() {
        assert!(Permission::Admin.satisfies(Permission::Write));
        assert!(Permission::Write.satisfies(Permission::Read));
        assert!(Permission::Read.satisfies(Permission::Read));
        assert!(!Permission::Read.satisfies(Permission::Write));
        assert!(!Permission::Write.satisfies(Permission::Admin));
    }
}
