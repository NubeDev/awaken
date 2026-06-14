//! RBAC user rows: create, list (by org), get, update, delete, and the
//! verifier's `user_by_subject` lookup. A user is an account keyed by its
//! verified token `subject`; `admin_level` is its admin tier. Backend dispatch;
//! SQLite body inline, Postgres body in [`super::postgres::users`]. See
//! `docs/design/authz-rbac.md`.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::AdminLevel;

use super::backend::Backend;
use super::codec::{ts_of, ts_to};
use super::{Result, Store, StoreError};

/// One `users` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct UserRecord {
    pub id: Uuid,
    /// Home org.
    pub org: String,
    /// The verified token subject (OIDC `sub` / PAT id) this user is identified by.
    pub subject: String,
    pub email: String,
    pub display_name: String,
    /// Admin tier; folded into the principal role at verify time.
    #[schema(value_type = String)]
    pub admin_level: AdminLevel,
    pub created_at: DateTime<Utc>,
}

pub(crate) const USER_COLS: &str =
    "id, org, subject, email, display_name, admin_level, created_at";

/// Row mapper shared with [`super::teams`] (the team-member join selects
/// [`USER_COLS`] in the same order).
pub(super) fn row_user_pub(row: &Row<'_>) -> rusqlite::Result<UserRecord> {
    row_user(row)
}

fn row_user(row: &Row<'_>) -> rusqlite::Result<UserRecord> {
    let level_raw: String = row.get(5)?;
    Ok(UserRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        subject: row.get(2)?,
        email: row.get(3)?,
        display_name: row.get(4)?,
        admin_level: AdminLevel::parse(&level_raw),
        created_at: ts_to(&row.get::<_, String>(6)?)?,
    })
}

impl Store {
    pub fn create_user(&self, user: &UserRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_user_sqlite(user),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::create_user(self, user),
        }
    }

    fn create_user_sqlite(&self, user: &UserRecord) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO users (id, org, subject, email, display_name, admin_level, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user.id,
                user.org,
                user.subject,
                user.email,
                user.display_name,
                user.admin_level.as_str(),
                ts_of(&user.created_at),
            ],
        )?;
        Ok(())
    }

    /// Users under an org, newest first.
    pub fn list_users(&self, org: &str) -> Result<Vec<UserRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_users_sqlite(org),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::list_users(self, org),
        }
    }

    fn list_users_sqlite(&self, org: &str) -> Result<Vec<UserRecord>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {USER_COLS} FROM users WHERE org = ?1 ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map(params![org], row_user)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_user(&self, id: Uuid) -> Result<UserRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_user_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::get_user(self, id),
        }
    }

    fn get_user_sqlite(&self, id: Uuid) -> Result<UserRecord> {
        self.sqlite_conn()?
            .query_row(
                &format!("SELECT {USER_COLS} FROM users WHERE id = ?1"),
                params![id],
                row_user,
            )
            .optional()?
            .ok_or(StoreError::NotFound("user"))
    }

    /// Look a user up by verified subject — the verifier's enrichment path.
    /// `None` when the subject maps to no user row (pure-token principal).
    pub fn user_by_subject(&self, subject: &str) -> Result<Option<UserRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.user_by_subject_sqlite(subject),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::user_by_subject(self, subject),
        }
    }

    fn user_by_subject_sqlite(&self, subject: &str) -> Result<Option<UserRecord>> {
        Ok(self
            .sqlite_conn()?
            .query_row(
                &format!("SELECT {USER_COLS} FROM users WHERE subject = ?1"),
                params![subject],
                row_user,
            )
            .optional()?)
    }

    /// True when the `users` table has no rows — used by the super-admin
    /// first-user bootstrap fallback.
    pub fn users_empty(&self) -> Result<bool> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n: i64 =
                    self.sqlite_conn()?
                        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
                Ok(n == 0)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::users_empty(self),
        }
    }

    /// Patch a user's mutable fields (`email`, `display_name`, `admin_level`).
    /// `org`/`subject` are identity and immutable. Returns the updated row.
    pub fn update_user(
        &self,
        id: Uuid,
        email: Option<&str>,
        display_name: Option<&str>,
        admin_level: Option<AdminLevel>,
    ) -> Result<UserRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.update_user_sqlite(id, email, display_name, admin_level),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::users::update_user(self, id, email, display_name, admin_level)
            }
        }
    }

    fn update_user_sqlite(
        &self,
        id: Uuid,
        email: Option<&str>,
        display_name: Option<&str>,
        admin_level: Option<AdminLevel>,
    ) -> Result<UserRecord> {
        let conn = self.sqlite_conn()?;
        let n = conn.execute(
            "UPDATE users SET email = COALESCE(?2, email), \
             display_name = COALESCE(?3, display_name), \
             admin_level = COALESCE(?4, admin_level) WHERE id = ?1",
            params![
                id,
                email,
                display_name,
                admin_level.map(|l| l.as_str()),
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("user"));
        }
        conn.query_row(
            &format!("SELECT {USER_COLS} FROM users WHERE id = ?1"),
            params![id],
            row_user,
        )
        .map_err(Into::into)
    }

    pub fn delete_user(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_user_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::users::delete_user(self, id),
        }
    }

    fn delete_user_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM users WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("user"));
        }
        Ok(())
    }
}
