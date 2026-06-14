//! Team rows + memberships, Postgres backend. Mirrors [`super::super::teams`].

use uuid::Uuid;

use super::super::codec::ts_of;
use super::super::teams::{TeamRecord, TEAM_COLS};
use super::super::users::{UserRecord, USER_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{ts_col, uuid_of};
use super::users::user_of;

fn team_of(row: &postgres::Row) -> Result<TeamRecord> {
    Ok(TeamRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        slug: row.get(2),
        name: row.get(3),
        created_at: ts_col(row, 4)?,
    })
}

pub(crate) fn create_team(store: &Store, team: &TeamRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO teams (id, org, slug, name, created_at) VALUES ($1, $2, $3, $4, $5)",
        &[
            &team.id.to_string(),
            &team.org,
            &team.slug,
            &team.name,
            &ts_of(&team.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_teams(store: &Store, org: &str) -> Result<Vec<TeamRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {TEAM_COLS} FROM teams WHERE org = $1 ORDER BY slug");
    client.query(sql.as_str(), &[&org])?.iter().map(team_of).collect()
}

pub(crate) fn get_team(store: &Store, id: Uuid) -> Result<TeamRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {TEAM_COLS} FROM teams WHERE id = $1");
    match client.query_opt(sql.as_str(), &[&id.to_string()])? {
        Some(row) => team_of(&row),
        None => Err(StoreError::NotFound("team")),
    }
}

pub(crate) fn update_team(store: &Store, id: Uuid, name: Option<&str>) -> Result<TeamRecord> {
    let mut client = store.postgres_conn()?;
    let n = client.execute(
        "UPDATE teams SET name = COALESCE($2, name) WHERE id = $1",
        &[&id.to_string(), &name],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("team"));
    }
    get_team(store, id)
}

pub(crate) fn delete_team(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM teams WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("team"));
    }
    Ok(())
}

pub(crate) fn add_team_member(store: &Store, team_id: Uuid, user_id: Uuid) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO memberships (user_id, team_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        &[&user_id.to_string(), &team_id.to_string()],
    )?;
    Ok(())
}

pub(crate) fn remove_team_member(store: &Store, team_id: Uuid, user_id: Uuid) -> Result<()> {
    let n = store.postgres_conn()?.execute(
        "DELETE FROM memberships WHERE user_id = $1 AND team_id = $2",
        &[&user_id.to_string(), &team_id.to_string()],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("membership"));
    }
    Ok(())
}

pub(crate) fn list_team_members(store: &Store, team_id: Uuid) -> Result<Vec<UserRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {USER_COLS} FROM users u JOIN memberships m ON m.user_id = u.id \
         WHERE m.team_id = $1 ORDER BY u.created_at DESC"
    );
    client
        .query(sql.as_str(), &[&team_id.to_string()])?
        .iter()
        .map(user_of)
        .collect()
}

pub(crate) fn team_ids_for_user(store: &Store, user_id: Uuid) -> Result<Vec<Uuid>> {
    let mut client = store.postgres_conn()?;
    let rows = client.query(
        "SELECT team_id FROM memberships WHERE user_id = $1",
        &[&user_id.to_string()],
    )?;
    rows.iter().map(|r| uuid_of(r, 0)).collect()
}
