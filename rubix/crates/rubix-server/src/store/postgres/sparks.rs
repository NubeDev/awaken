//! Spark rows, Postgres backend. Mirrors [`super::super::sparks`].

use rubix_core::Spark;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::{Result, Store, StoreError};
use super::codec::{json_col, require, token_enum, ts_col, uuid_of};

fn spark_of(row: &postgres::Row) -> Result<Spark> {
    Ok(Spark {
        id: uuid_of(row, 0)?,
        site_id: uuid_of(row, 1)?,
        rule: row.get(2),
        severity: token_enum(row, 3)?,
        message: row.get(4),
        point_ids: json_col(row, 5)?,
        ts: ts_col(row, 6)?,
        acknowledged: row.get(7),
    })
}

pub(crate) fn create_spark(store: &Store, spark: &Spark) -> Result<()> {
    let mut client = store.postgres_conn()?;
    require(&mut *client, "sites", "site", spark.site_id)?;
    let severity = json_of(&spark.severity).trim_matches('"').to_string();
    client.execute(
        "INSERT INTO sparks (id, site_id, rule, severity, message, point_ids, ts, acknowledged) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &spark.id.to_string(),
            &spark.site_id.to_string(),
            &spark.rule,
            &severity,
            &spark.message,
            &json_of(&spark.point_ids),
            &ts_of(&spark.ts),
            &spark.acknowledged,
        ],
    )?;
    Ok(())
}

pub(crate) fn list_sparks(
    store: &Store,
    site_id: Option<Uuid>,
    rule: Option<&str>,
    acknowledged: Option<bool>,
) -> Result<Vec<Spark>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let rows = client.query(
        "SELECT id, site_id, rule, severity, message, point_ids, ts, acknowledged \
         FROM sparks WHERE ($1::text IS NULL OR site_id = $1) AND ($2::text IS NULL OR rule = $2) \
           AND ($3::bool IS NULL OR acknowledged = $3) ORDER BY ts DESC",
        &[&site, &rule, &acknowledged],
    )?;
    rows.iter().map(spark_of).collect()
}

pub(crate) fn ack_spark(store: &Store, id: Uuid) -> Result<()> {
    let n = store.postgres_conn()?.execute(
        "UPDATE sparks SET acknowledged = TRUE WHERE id = $1",
        &[&id.to_string()],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("spark"));
    }
    Ok(())
}
