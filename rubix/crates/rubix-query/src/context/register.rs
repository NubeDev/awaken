//! Build a `SessionContext` with the canonical tables registered live.

use std::sync::Arc;

use datafusion::prelude::SessionContext;

use super::tables::CANONICAL;
use super::QueryEngine;
use crate::error::QueryError;
use crate::his::HisTable;
use crate::provider::SqliteTable;

impl QueryEngine {
    /// Build a fresh context with each canonical table registered under its
    /// bare name (so `SELECT * FROM points` resolves directly). Schema is read
    /// from SQLite at call time, so empty tables still expose their columns.
    /// Derived views (e.g. `points_cur`) are registered over those tables.
    pub(crate) async fn session(&self) -> Result<SessionContext, QueryError> {
        let ctx = SessionContext::new();
        for &table in CANONICAL {
            // `his` resolves through the two-tier union provider when a Parquet
            // cold tier is attached; every other canonical table (and `his`
            // without a tier) is the live SQLite provider.
            if table == "his" {
                if let Some(tier) = &self.his_tier {
                    let provider = HisTable::new(self.pool.clone(), tier.store());
                    ctx.register_table(table, Arc::new(provider))
                        .map_err(|source| QueryError::Register { table, source })?;
                    continue;
                }
            }
            let provider = SqliteTable::try_new(self.pool.clone(), table)?;
            ctx.register_table(table, Arc::new(provider))
                .map_err(|source| QueryError::Register { table, source })?;
        }
        self.register_views(&ctx).await?;
        Ok(ctx)
    }

    /// Register derived SQL views over the base tables. `points_cur` flattens
    /// the per-point effective current value (`cur_value`/`cur_ts`) and joins
    /// in the site/equip path so a dashboard can `SELECT * FROM points_cur`
    /// without rebuilding the keyexpr.
    async fn register_views(&self, ctx: &SessionContext) -> Result<(), QueryError> {
        ctx.sql(POINTS_CUR_VIEW)
            .await
            .map_err(|source| QueryError::Register {
                table: "points_cur",
                source,
            })?;
        Ok(())
    }
}

/// `points_cur`: one row per point with its effective current value and the
/// resolved keyexpr (`{org}/{site}/{equip-path}/{point}`). `cur_value` is the
/// JSON-encoded `PointValue` as stored; `cur_ts` is the last-change timestamp.
const POINTS_CUR_VIEW: &str = "\
CREATE VIEW points_cur AS \
SELECT p.id AS point_id, \
       s.org || '/' || s.slug || '/' || e.path || '/' || p.slug AS keyexpr, \
       p.kind AS kind, \
       p.unit AS unit, \
       p.cur_value AS cur_value, \
       p.cur_ts AS cur_ts \
FROM points p \
JOIN equips e ON p.equip_id = e.id \
JOIN sites s ON e.site_id = s.id";
