//! Map the standalone `rubix_rules::Severity` onto the canonical
//! `rubix_core::SparkSeverity`.
//!
//! `rubix-rules` is standalone and carries its own `info`/`warning`/`fault`
//! mirror on purpose (it depends on no other rubix crate). The map lives here,
//! in the integrating crate, so a flagged rule result drives a finding at the
//! canonical severity rather than at `emit_spark`'s static config — the rule's
//! decision is authoritative. The two enums share the same three variants and
//! the same lowercase wire string, so the map is total and lossless.

use rubix_core::SparkSeverity;
use rubix_rules::Severity;

/// The canonical spark severity for a rule result's severity.
pub fn spark_severity(severity: Severity) -> SparkSeverity {
    match severity {
        Severity::Info => SparkSeverity::Info,
        Severity::Warning => SparkSeverity::Warning,
        Severity::Fault => SparkSeverity::Fault,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_maps_to_its_canonical_twin() {
        assert_eq!(spark_severity(Severity::Info), SparkSeverity::Info);
        assert_eq!(spark_severity(Severity::Warning), SparkSeverity::Warning);
        assert_eq!(spark_severity(Severity::Fault), SparkSeverity::Fault);
    }

    /// The map agrees with both enums' shared wire string, so a finding
    /// round-trips through either type without drifting.
    #[test]
    fn map_agrees_with_wire_string() {
        for s in [Severity::Info, Severity::Warning, Severity::Fault] {
            let mapped = spark_severity(s);
            let mapped_str = serde_json::to_value(mapped).unwrap();
            assert_eq!(mapped_str, serde_json::Value::String(s.as_str().into()));
        }
    }
}
