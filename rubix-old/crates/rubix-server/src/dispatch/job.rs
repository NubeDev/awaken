//! Render a spark finding into the job prompt that activates an agent run.
//! A dispatched run is a *job*, not a chat: the agent is handed the finding and
//! told to investigate and, within its gating, act — distinct from an operator
//! conversation.

use rubix_core::Spark;

/// The agent thread a spark's run uses. Keyed by spark id so a finding gets one
/// run and a re-delivery (zenoh is best-effort) lands on the same thread rather
/// than spawning a duplicate conversation.
pub fn thread_id(spark: &Spark) -> String {
    format!("spark-{}", spark.id)
}

/// The job prompt handed to the agent for a finding. States the rule, severity,
/// and message, and frames the task as investigate-then-act-within-gating.
pub fn prompt(spark: &Spark) -> String {
    format!(
        "A rule finding fired on the building and needs attention.\n\
         Rule: {rule}\n\
         Severity: {severity:?}\n\
         Finding: {message}\n\n\
         Investigate using the read and history tools, and take corrective action \
         through the command tool only if it is safe and within your priority \
         floor. Explain what you found and what you changed.",
        rule = spark.rule,
        severity = spark.severity,
        message = spark.message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rubix_core::SparkSeverity;
    use uuid::Uuid;

    fn spark() -> Spark {
        Spark {
            id: Uuid::nil(),
            site_id: Uuid::nil(),
            rule: "heat_cool_conflict".into(),
            severity: SparkSeverity::Fault,
            message: "AHU-3 heating and cooling at once".into(),
            point_ids: vec![],
            ts: Utc::now(),
            acknowledged: false,
        }
    }

    #[test]
    fn thread_id_is_keyed_by_spark_id() {
        assert_eq!(
            thread_id(&spark()),
            "spark-00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn prompt_carries_rule_and_message() {
        let p = prompt(&spark());
        assert!(p.contains("heat_cool_conflict"));
        assert!(p.contains("AHU-3 heating and cooling at once"));
        assert!(p.contains("Fault"));
    }
}
