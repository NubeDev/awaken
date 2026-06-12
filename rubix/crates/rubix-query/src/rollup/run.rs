//! Build and run a time-bucketed rollup over `his`.

use super::spec::RollupSpec;
use crate::error::QueryError;
use crate::sql::QueryRows;
use crate::QueryEngine;

/// Bucket origin: rollup buckets align to the Unix epoch so series from
/// different queries share boundaries.
const ORIGIN: &str = "TIMESTAMP '1970-01-01T00:00:00'";

impl QueryEngine {
    /// Roll up `his` into time buckets per point.
    ///
    /// Returns one row per (point_id, bucket) with the aggregate as `value`,
    /// ordered by point then bucket. Aggregate and interval are closed enums
    /// rendered to fixed SQL; point ids and bounds are validated and quoted, so
    /// no untrusted text reaches the statement unescaped.
    pub async fn his_rollup(&self, spec: &RollupSpec) -> Result<QueryRows, QueryError> {
        if spec.points.is_empty() {
            return Ok(Vec::new());
        }
        let points = quote_list(&spec.points)?;
        let mut sql = format!(
            "SELECT point_id, \
             date_bin({interval}, to_timestamp(ts), {ORIGIN}) AS bucket, \
             {agg} AS value, count(*) AS samples \
             FROM his WHERE point_id IN ({points})",
            interval = spec.interval.sql(),
            agg = spec.agg.sql(),
        );
        if let Some(start) = &spec.start {
            sql.push_str(&format!(" AND ts >= {}", quote(start)?));
        }
        if let Some(end) = &spec.end {
            sql.push_str(&format!(" AND ts < {}", quote(end)?));
        }
        sql.push_str(" GROUP BY point_id, bucket ORDER BY point_id, bucket");
        self.query(&sql).await
    }
}

/// Quote a string as a SQL literal, rejecting embedded quotes outright rather
/// than escaping — point ids and timestamps never contain them.
fn quote(s: &str) -> Result<String, QueryError> {
    if s.contains('\'') || s.contains('\0') {
        return Err(QueryError::Backend(format!("illegal literal: {s:?}")));
    }
    Ok(format!("'{s}'"))
}

fn quote_list(items: &[String]) -> Result<String, QueryError> {
    let quoted = items
        .iter()
        .map(|s| quote(s))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quoted.join(", "))
}
