//! Publish a rule finding on `{org}/{site}/spark/{rule}/{id}`.

use rubix_core::Spark;

use super::ZenohBus;

impl ZenohBus {
    /// Publish a spark on its rule keyexpr so cloud subscribers (alerting,
    /// agent dispatch) observe findings live, per the `spark` keyexpr scheme in
    /// STACK-DEISGN.md. The `{id}` leaf disambiguates concurrent findings of the
    /// same rule. Payload is the JSON-encoded [`Spark`]. Best-effort: the spark
    /// is already persisted, so a publish failure is logged, not propagated.
    pub async fn publish_spark(&self, org: &str, site_slug: &str, spark: &Spark) {
        let key = format!(
            "{org}/{site_slug}/spark/{rule}/{id}",
            rule = spark.rule,
            id = spark.id
        );
        let payload = match serde_json::to_vec(spark) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(key, error = %e, "encode spark payload");
                return;
            }
        };
        if let Err(e) = self.session().put(&key, payload).await {
            tracing::warn!(key, error = %e, "publish spark");
        }
    }
}
