//! Token rows, Postgres backend. Mirrors [`super::super::tokens`]. The scope
//! columns are nullable TEXT; the role is a bare token, matching the SQLite
//! path.

use crate::auth::{Role, Scope, TokenRecord};

use super::super::codec::ts_of;
use super::super::tokens::TOKEN_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::ts_col;

fn token_of(row: &postgres::Row) -> Result<TokenRecord> {
    let role_raw: String = row.get(3);
    let revoked: Option<String> = row.get(8);
    Ok(TokenRecord {
        id: row.get(0),
        secret_hash: row.get(1),
        name: row.get(2),
        role: Role::parse(&role_raw)
            .ok_or_else(|| StoreError::Db(anyhow::anyhow!("unknown token role `{role_raw}`")))?,
        scope: Scope {
            org: row.get(4),
            team: row.get(5),
            site: row.get(6),
        },
        created_at: ts_col(row, 7)?,
        revoked_at: match revoked {
            Some(ts) => Some(
                super::super::codec::ts_to(&ts)
                    .map_err(|e| StoreError::Db(anyhow::anyhow!("bad revoked_at: {e}")))?,
            ),
            None => None,
        },
    })
}

pub(crate) fn create_token(store: &Store, token: &TokenRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO tokens (id, secret_hash, name, role, scope_org, scope_team, \
         scope_site, created_at, revoked_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &token.id,
            &token.secret_hash,
            &token.name,
            &token.role.as_str().to_string(),
            &token.scope.org,
            &token.scope.team,
            &token.scope.site,
            &ts_of(&token.created_at),
            &token.revoked_at.as_ref().map(ts_of),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_tokens(store: &Store) -> Result<Vec<TokenRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {TOKEN_COLS} FROM tokens ORDER BY created_at DESC");
    let rows = client.query(sql.as_str(), &[])?;
    rows.iter().map(token_of).collect()
}

pub(crate) fn token_by_hash(store: &Store, secret_hash: &str) -> Result<Option<TokenRecord>> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {TOKEN_COLS} FROM tokens WHERE secret_hash = $1");
    match client.query_opt(sql.as_str(), &[&secret_hash])? {
        Some(row) => Ok(Some(token_of(&row)?)),
        None => Ok(None),
    }
}

pub(crate) fn revoke_token(store: &Store, id: &str) -> Result<()> {
    let mut client = store.postgres_conn()?;
    let affected = client.execute(
        "UPDATE tokens SET revoked_at = $2 WHERE id = $1 AND revoked_at IS NULL",
        &[&id, &ts_of(&chrono::Utc::now())],
    )?;
    if affected == 0 && client.query_opt("SELECT 1 FROM tokens WHERE id = $1", &[&id])?.is_none() {
        return Err(StoreError::NotFound("token"));
    }
    Ok(())
}
