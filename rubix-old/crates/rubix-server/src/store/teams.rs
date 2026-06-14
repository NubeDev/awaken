//! RBAC team rows + memberships: team CRUD, plus the user↔team join
//! (`memberships`). A team is a named group within an org; grants can target a
//! team so its members inherit the grant. Backend dispatch; SQLite body inline,
//! Postgres body in [`super::postgres::teams`]. See `docs/design/authz-rbac.md`.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{ts_of, ts_to};
use super::{Result, Store, StoreError};

/// One `teams` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct TeamRecord {
    pub id: Uuid,
    pub org: String,
    pub slug: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

pub(crate) const TEAM_COLS: &str = "id, org, slug, name, created_at";

fn row_team(row: &Row<'_>) -> rusqlite::Result<TeamRecord> {
    Ok(TeamRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        slug: row.get(2)?,
        name: row.get(3)?,
        created_at: ts_to(&row.get::<_, String>(4)?)?,
    })
}

impl Store {
    pub fn create_team(&self, team: &TeamRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_team_sqlite(team),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::create_team(self, team),
        }
    }

    fn create_team_sqlite(&self, team: &TeamRecord) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO teams (id, org, slug, name, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![team.id, team.org, team.slug, team.name, ts_of(&team.created_at)],
        )?;
        Ok(())
    }

    pub fn list_teams(&self, org: &str) -> Result<Vec<TeamRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_teams_sqlite(org),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::list_teams(self, org),
        }
    }

    fn list_teams_sqlite(&self, org: &str) -> Result<Vec<TeamRecord>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TEAM_COLS} FROM teams WHERE org = ?1 ORDER BY slug"
        ))?;
        let rows = stmt.query_map(params![org], row_team)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_team(&self, id: Uuid) -> Result<TeamRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.get_team_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::get_team(self, id),
        }
    }

    fn get_team_sqlite(&self, id: Uuid) -> Result<TeamRecord> {
        self.sqlite_conn()?
            .query_row(
                &format!("SELECT {TEAM_COLS} FROM teams WHERE id = ?1"),
                params![id],
                row_team,
            )
            .optional()?
            .ok_or(StoreError::NotFound("team"))
    }

    /// Patch a team's mutable field (`name`). `org`/`slug` are identity.
    pub fn update_team(&self, id: Uuid, name: Option<&str>) -> Result<TeamRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self.update_team_sqlite(id, name),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::update_team(self, id, name),
        }
    }

    fn update_team_sqlite(&self, id: Uuid, name: Option<&str>) -> Result<TeamRecord> {
        let conn = self.sqlite_conn()?;
        let n = conn.execute(
            "UPDATE teams SET name = COALESCE(?2, name) WHERE id = ?1",
            params![id, name],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("team"));
        }
        conn.query_row(
            &format!("SELECT {TEAM_COLS} FROM teams WHERE id = ?1"),
            params![id],
            row_team,
        )
        .map_err(Into::into)
    }

    pub fn delete_team(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.delete_team_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::delete_team(self, id),
        }
    }

    fn delete_team_sqlite(&self, id: Uuid) -> Result<()> {
        let n = self
            .sqlite_conn()?
            .execute("DELETE FROM teams WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound("team"));
        }
        Ok(())
    }

    /// Add a user to a team (idempotent; a repeat add is a no-op success).
    pub fn add_team_member(&self, team_id: Uuid, user_id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "INSERT OR IGNORE INTO memberships (user_id, team_id) VALUES (?1, ?2)",
                    params![user_id, team_id],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::add_team_member(self, team_id, user_id),
        }
    }

    /// Remove a user from a team. Fails if the membership is absent.
    pub fn remove_team_member(&self, team_id: Uuid, user_id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "DELETE FROM memberships WHERE user_id = ?1 AND team_id = ?2",
                    params![user_id, team_id],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("membership"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::teams::remove_team_member(self, team_id, user_id)
            }
        }
    }

    /// The users belonging to a team.
    pub fn list_team_members(&self, team_id: Uuid) -> Result<Vec<super::users::UserRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_team_members_sqlite(team_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::list_team_members(self, team_id),
        }
    }

    fn list_team_members_sqlite(&self, team_id: Uuid) -> Result<Vec<super::users::UserRecord>> {
        let conn = self.sqlite_conn()?;
        let cols = super::users::USER_COLS;
        let mut stmt = conn.prepare(&format!(
            "SELECT {cols} FROM users u \
             JOIN memberships m ON m.user_id = u.id \
             WHERE m.team_id = ?1 ORDER BY u.created_at DESC"
        ))?;
        let rows = stmt.query_map(params![team_id], super::users::row_user_pub)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The team ids a user belongs to — the verifier's enrichment path.
    pub fn team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let mut stmt =
                    conn.prepare("SELECT team_id FROM memberships WHERE user_id = ?1")?;
                let rows = stmt.query_map(params![user_id], |r| r.get::<_, Uuid>(0))?;
                Ok(rows.collect::<rusqlite::Result<_>>()?)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::teams::team_ids_for_user(self, user_id),
        }
    }
}
