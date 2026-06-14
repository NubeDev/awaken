//! User rows, Postgres backend. Mirrors [`super::super::users`]. Ids are TEXT
//! (canonical UUID strings); `admin_level` is a bare token.

use uuid::Uuid;

use crate::auth::AdminLevel;

use super::super::codec::ts_of;
use super::super::users::{UserRecord, USER_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{ts_col, uuid_of};

pub(super) fn user_of(row: &postgres::Row) -> Result<UserRecord> {
    let level_raw: String = row.get(5);
    Ok(UserRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        subject: row.get(2),
        email: row.get(3),
        display_name: row.get(4),
        admin_level: AdminLevel::parse(&level_raw),
        created_at: ts_col(row, 6)?,
    })
}

pub(crate) fn create_user(store: &Store, user: &UserRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO users (id, org, subject, email, display_name, admin_level, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[
            &user.id.to_string(),
            &user.org,
            &user.subject,
            &user.email,
            &user.display_name,
            &user.admin_level.as_str().to_string(),
            &ts_of(&user.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_users(store: &Store, org: &str) -> Result<Vec<UserRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {USER_COLS} FROM users WHERE org = $1 ORDER BY created_at DESC");
    client.query(sql.as_str(), &[&org])?.iter().map(user_of).collect()
}

pub(crate) fn get_user(store: &Store, id: Uuid) -> Result<UserRecord> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {USER_COLS} FROM users WHERE id = $1");
    match client.query_opt(sql.as_str(), &[&id.to_string()])? {
        Some(row) => user_of(&row),
        None => Err(StoreError::NotFound("user")),
    }
}

pub(crate) fn user_by_subject(store: &Store, subject: &str) -> Result<Option<UserRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {USER_COLS} FROM users WHERE subject = $1");
    match client.query_opt(sql.as_str(), &[&subject])? {
        Some(row) => Ok(Some(user_of(&row)?)),
        None => Ok(None),
    }
}

pub(crate) fn users_empty(store: &Store) -> Result<bool> {
    let row = store
        .postgres_conn()?
        .query_one("SELECT COUNT(*)::BIGINT FROM users", &[])?;
    let n: i64 = row.get(0);
    Ok(n == 0)
}

pub(crate) fn update_user(
    store: &Store,
    id: Uuid,
    email: Option<&str>,
    display_name: Option<&str>,
    admin_level: Option<AdminLevel>,
) -> Result<UserRecord> {
    let mut client = store.postgres_conn()?;
    let level = admin_level.map(|l| l.as_str().to_string());
    let n = client.execute(
        "UPDATE users SET email = COALESCE($2, email), \
         display_name = COALESCE($3, display_name), \
         admin_level = COALESCE($4, admin_level) WHERE id = $1",
        &[&id.to_string(), &email, &display_name, &level],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("user"));
    }
    get_user(store, id)
}

pub(crate) fn delete_user(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM users WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("user"));
    }
    Ok(())
}
