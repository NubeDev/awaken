//! Spark finding rows: create, list, acknowledge. Backend dispatch; SQLite body
//! inline, Postgres body in [`super::postgres::sparks`].

use rubix_core::{Spark, SparkSeverity};
use rusqlite::params;
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

impl Store {
    pub fn create_spark(&self, spark: &Spark) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_spark_sqlite(spark),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sparks::create_spark(self, spark),
        }
    }

    fn create_spark_sqlite(&self, spark: &Spark) -> Result<()> {
        let conn = self.sqlite_conn()?;
        Self::require_site(&conn, spark.site_id)?;
        let severity = json_of(&spark.severity);
        conn.execute(
            "INSERT INTO sparks (id, site_id, rule, severity, message, point_ids, ts, acknowledged) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                spark.id,
                spark.site_id,
                spark.rule,
                severity.trim_matches('"'),
                spark.message,
                json_of(&spark.point_ids),
                ts_of(&spark.ts),
                spark.acknowledged
            ],
        )?;
        Ok(())
    }

    pub fn list_sparks(
        &self,
        site_id: Option<Uuid>,
        rule: Option<&str>,
        acknowledged: Option<bool>,
    ) -> Result<Vec<Spark>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_sparks_sqlite(site_id, rule, acknowledged),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::sparks::list_sparks(self, site_id, rule, acknowledged)
            }
        }
    }

    fn list_sparks_sqlite(
        &self,
        site_id: Option<Uuid>,
        rule: Option<&str>,
        acknowledged: Option<bool>,
    ) -> Result<Vec<Spark>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, site_id, rule, severity, message, point_ids, ts, acknowledged \
             FROM sparks WHERE (?1 IS NULL OR site_id = ?1) AND (?2 IS NULL OR rule = ?2) \
               AND (?3 IS NULL OR acknowledged = ?3) ORDER BY ts DESC",
        )?;
        let rows = stmt.query_map(params![site_id, rule, acknowledged], |row| {
            Ok(Spark {
                id: row.get(0)?,
                site_id: row.get(1)?,
                rule: row.get(2)?,
                severity: json_to::<SparkSeverity>(&format!("\"{}\"", row.get::<_, String>(3)?))?,
                message: row.get(4)?,
                point_ids: json_to(&row.get::<_, String>(5)?)?,
                ts: ts_to(&row.get::<_, String>(6)?)?,
                acknowledged: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn ack_spark(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "UPDATE sparks SET acknowledged = 1 WHERE id = ?1",
                    params![id],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("spark"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::sparks::ack_spark(self, id),
        }
    }
}
