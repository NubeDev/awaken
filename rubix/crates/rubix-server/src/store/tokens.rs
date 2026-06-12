//! PAT / service-account rows: issue, list, look up by secret hash (the
//! verifier's hot path), and revoke. The store holds only the SHA-256 of the
//! secret — see [`crate::auth::pat`]. Backend dispatch; SQLite body inline,
//! Postgres body in [`super::postgres::tokens`].

use rusqlite::{params, OptionalExtension, Row};

use crate::auth::{Role, Scope, TokenRecord};

use super::backend::Backend;
use super::codec::{ts_of, ts_to};
use super::{Result, Store, StoreError};

pub(crate) const TOKEN_COLS: &str =
    "id, secret_hash, name, role, scope_org, scope_team, scope_site, created_at, revoked_at";

fn row_token(row: &Row<'_>) -> rusqlite::Result<TokenRecord> {
    let role_raw: String = row.get(3)?;
    let revoked_raw: Option<String> = row.get(8)?;
    Ok(TokenRecord {
        id: row.get(0)?,
        secret_hash: row.get(1)?,
        name: row.get(2)?,
        role: Role::parse(&role_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                format!("unknown token role `{role_raw}`").into(),
            )
        })?,
        scope: Scope {
            org: row.get(4)?,
            team: row.get(5)?,
            site: row.get(6)?,
        },
        created_at: ts_to(&row.get::<_, String>(7)?)?,
        revoked_at: match revoked_raw {
            Some(ts) => Some(ts_to(&ts)?),
            None => None,
        },
    })
}

impl Store {
    /// Persist a freshly minted token.
    pub fn create_token(&self, token: &TokenRecord) -> Result<()> {
        token
            .scope
            .validate()
            .map_err(|e| StoreError::Invalid(e.to_string()))?;
        match &self.backend {
            Backend::Sqlite(_) => self.create_token_sqlite(token),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::tokens::create_token(self, token),
        }
    }

    fn create_token_sqlite(&self, token: &TokenRecord) -> Result<()> {
        self.sqlite_conn()?.execute(
            "INSERT INTO tokens (id, secret_hash, name, role, scope_org, scope_team, \
             scope_site, created_at, revoked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                token.id,
                token.secret_hash,
                token.name,
                token.role.as_str(),
                token.scope.org,
                token.scope.team,
                token.scope.site,
                ts_of(&token.created_at),
                token.revoked_at.as_ref().map(ts_of),
            ],
        )?;
        Ok(())
    }

    /// All tokens, newest first (revoked rows included, flagged by `revoked_at`).
    pub fn list_tokens(&self) -> Result<Vec<TokenRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_tokens_sqlite(),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::tokens::list_tokens(self),
        }
    }

    fn list_tokens_sqlite(&self) -> Result<Vec<TokenRecord>> {
        let conn = self.sqlite_conn()?;
        let mut stmt =
            conn.prepare(&format!("SELECT {TOKEN_COLS} FROM tokens ORDER BY created_at DESC"))?;
        let rows = stmt.query_map([], row_token)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Look a token up by its secret hash. The verifier's hot path: a single
    /// indexed read on the `UNIQUE (secret_hash)` column. Returns `None` for an
    /// unknown hash so an invalid PAT fails closed.
    pub fn token_by_hash(&self, secret_hash: &str) -> Result<Option<TokenRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => self.token_by_hash_sqlite(secret_hash),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::tokens::token_by_hash(self, secret_hash),
        }
    }

    fn token_by_hash_sqlite(&self, secret_hash: &str) -> Result<Option<TokenRecord>> {
        Ok(self
            .sqlite_conn()?
            .query_row(
                &format!("SELECT {TOKEN_COLS} FROM tokens WHERE secret_hash = ?1"),
                params![secret_hash],
                row_token,
            )
            .optional()?)
    }

    /// Revoke a token by id. Idempotent on an already-revoked row (the timestamp
    /// is left at its first revocation). Fails if the token is absent.
    pub fn revoke_token(&self, id: &str) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.revoke_token_sqlite(id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::tokens::revoke_token(self, id),
        }
    }

    fn revoke_token_sqlite(&self, id: &str) -> Result<()> {
        let affected = self.sqlite_conn()?.execute(
            "UPDATE tokens SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
            params![id, ts_of(&chrono::Utc::now())],
        )?;
        if affected == 0 {
            // Either absent or already revoked; distinguish so a double-revoke is
            // a no-op success, not a spurious 404.
            self.sqlite_conn()?
                .query_row("SELECT 1 FROM tokens WHERE id = ?1", params![id], |_| Ok(()))
                .optional()?
                .ok_or(StoreError::NotFound("token"))?;
        }
        Ok(())
    }
}
