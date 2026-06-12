//! Build a `SessionContext` confined to one tenant `{org}/{site}`.
//!
//! Each canonical table is registered as a *view over its raw provider* that is
//! already filtered to the scope, and the raw providers are never registered
//! under a nameable table. So a scoped agent's `SELECT * FROM points` resolves
//! to a tenant-filtered view, there is no base table to name to escape the
//! tenant, and the confinement does not depend on parsing the agent's SQL.

use datafusion::common::{Column, JoinType};
use datafusion::logical_expr::{col, lit, Expr};
use datafusion::prelude::SessionContext;

use super::scope::QueryScope;
use super::QueryEngine;
use crate::error::QueryError;

impl QueryEngine {
    /// Build a context whose canonical tables are views filtered to `scope`.
    ///
    /// The same set of names resolves as in an unscoped [`session`](Self::session)
    /// — `sites`, `equips`, `points`, `his`, `sparks`, and the `points_cur`
    /// view — but every one only exposes rows under the scope's `{org}/{site}`,
    /// and the raw providers are not nameable.
    pub(crate) async fn scoped_session(
        &self,
        scope: &QueryScope,
    ) -> Result<SessionContext, QueryError> {
        let ctx = SessionContext::new();

        // `sites` filtered to the one tenant row; every other view descends from
        // it by a left-semi join on the owning id, which keeps only the child
        // table's columns (so the overlapping `id`/`created_at`/`tags` names
        // never collide) and admits a row only when its owner is in scope. The
        // join sides are aliased so the otherwise-ambiguous `id` is qualified.
        let sites = self
            .canonical_dataframe(&ctx, "sites")?
            .filter(
                col("org")
                    .eq(lit(scope.org()))
                    .and(col("slug").eq(lit(scope.site()))),
            )
            .map_err(register_err)?
            .alias("scoped_sites")
            .map_err(register_err)?;
        ctx.register_table("sites", sites.clone().into_view())
            .map_err(register_err)?;

        let equips = self
            .semijoin(&ctx, "equips", "scoped_equips", "site_id", sites, "scoped_sites")
            .map_err(register_err)?;
        ctx.register_table("equips", equips.clone().into_view())
            .map_err(register_err)?;

        let points = self
            .semijoin(&ctx, "points", "scoped_points", "equip_id", equips, "scoped_equips")
            .map_err(register_err)?;
        ctx.register_table("points", points.clone().into_view())
            .map_err(register_err)?;

        let his = self
            .semijoin(&ctx, "his", "scoped_his", "point_id", points, "scoped_points")
            .map_err(register_err)?;
        ctx.register_table("his", his.into_view())
            .map_err(register_err)?;

        // `sparks` carries its own `site_id`, so it filters on the scoped
        // `sites` view directly.
        let scoped_sites = ctx
            .table("sites")
            .await
            .map_err(register_err)?
            .alias("scoped_sites")
            .map_err(register_err)?;
        let sparks = self
            .semijoin(&ctx, "sparks", "scoped_sparks", "site_id", scoped_sites, "scoped_sites")
            .map_err(register_err)?;
        ctx.register_table("sparks", sparks.into_view())
            .map_err(register_err)?;

        // `points_cur` reads the already-scoped `points`/`equips`/`sites` views,
        // so it is tenant-filtered for free.
        ctx.sql(POINTS_CUR_VIEW).await.map_err(register_err)?;

        Ok(ctx)
    }

    /// A child canonical table left-semi-joined to an in-scope `parent`
    /// dataframe on `child.fk == parent.id`, keeping only the child's rows whose
    /// owner is in scope (and only the child's columns). Both sides are aliased
    /// so the shared `id` column is unambiguous in the join predicate.
    fn semijoin(
        &self,
        ctx: &SessionContext,
        child_table: &'static str,
        child_alias: &str,
        fk: &str,
        parent: datafusion::dataframe::DataFrame,
        parent_alias: &str,
    ) -> Result<datafusion::dataframe::DataFrame, datafusion::error::DataFusionError> {
        let child = self
            .canonical_dataframe(ctx, child_table)
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?
            .alias(child_alias)?;
        let on = qualified(child_alias, fk).eq(qualified(parent_alias, "id"));
        child.join_on(parent, JoinType::LeftSemi, [on])
    }
}

/// A column reference qualified by its relation alias (`alias.column`).
fn qualified(alias: &str, column: &str) -> Expr {
    Expr::Column(Column::new(Some(alias.to_string()), column))
}

fn register_err(source: datafusion::error::DataFusionError) -> QueryError {
    QueryError::Register {
        table: "scoped view",
        source,
    }
}

/// `points_cur` over the scoped canonical views (same shape as the unscoped
/// derived view, but inheriting the tenant filter from `points`/`equips`/`sites`).
const POINTS_CUR_VIEW: &str = "\
CREATE VIEW points_cur AS \
SELECT p.id AS point_id, \
       s.org || '/' || s.slug || '/' || e.path || '/' || p.slug AS keyexpr, \
       p.kind AS kind, p.unit AS unit, \
       p.cur_value AS cur_value, p.cur_ts AS cur_ts \
FROM points p \
JOIN equips e ON p.equip_id = e.id \
JOIN sites s ON e.site_id = s.id";
