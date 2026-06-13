//! Stored-rule rows, Postgres backend. Mirrors [`super::super::rules`].

use rubix_rules::ParamSchema;

use super::super::codec::{json_of, ts_of};
use super::super::rules::{RuleRecord, RULE_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{json_col, ts_col, uuid_of};

fn rule_of(row: &postgres::Row) -> Result<RuleRecord> {
    Ok(RuleRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        name: row.get(2),
        script: row.get(3),
        params: json_col(row, 4)?,
        created_at: ts_col(row, 5)?,
    })
}

pub(crate) fn create_rule(store: &Store, rule: &RuleRecord) -> Result<()> {
    store.postgres_conn()?.execute(
        "INSERT INTO rules (id, org, name, script, params, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &rule.id.to_string(),
            &rule.org,
            &rule.name,
            &rule.script,
            &json_of(&rule.params),
            &ts_of(&rule.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_rules(store: &Store, org: &str) -> Result<Vec<RuleRecord>> {
    let rows = store.postgres_conn()?.query(
        &format!("SELECT {RULE_COLS} FROM rules WHERE org = $1 ORDER BY created_at DESC"),
        &[&org],
    )?;
    rows.iter().map(rule_of).collect()
}

pub(crate) fn load_rule(store: &Store, org: &str, name: &str) -> Result<RuleRecord> {
    let row = store
        .postgres_conn()?
        .query_opt(
            &format!("SELECT {RULE_COLS} FROM rules WHERE org = $1 AND name = $2"),
            &[&org, &name],
        )?
        .ok_or(StoreError::NotFound("rule"))?;
    rule_of(&row)
}

pub(crate) fn update_rule(
    store: &Store,
    org: &str,
    name: &str,
    script: &str,
    params: &ParamSchema,
) -> Result<RuleRecord> {
    let n = store.postgres_conn()?.execute(
        "UPDATE rules SET script = $3, params = $4 WHERE org = $1 AND name = $2",
        &[&org, &name, &script, &json_of(params)],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("rule"));
    }
    load_rule(store, org, name)
}

pub(crate) fn delete_rule(store: &Store, org: &str, name: &str) -> Result<()> {
    let n = store.postgres_conn()?.execute(
        "DELETE FROM rules WHERE org = $1 AND name = $2",
        &[&org, &name],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("rule"));
    }
    Ok(())
}
