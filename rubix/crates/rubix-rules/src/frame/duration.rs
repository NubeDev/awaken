//! Parse a human duration (`"1h"`, `"15m"`, `"30s"`, `"7d"`) into seconds.
//!
//! `rolling_*` windows and `resample` intervals are time-durations, not row
//! counts (sensor data is irregular, so a row-count window is almost never what
//! a building-analytics author means — the design pins this). Both primitives
//! parse their duration string here into a whole number of seconds for use in a
//! window `RANGE` frame and in `date_bin`.

use crate::error::RuleError;

/// Parse `spec` (e.g. `"1h"`) into seconds. Supports `s`, `m`, `h`, `d`.
pub(crate) fn parse_seconds(spec: &str) -> Result<i64, RuleError> {
    let spec = spec.trim();
    let (num, unit) = spec.split_at(
        spec.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| RuleError::Runtime(format!("duration `{spec}`: missing unit")))?,
    );
    let n: i64 = num
        .parse()
        .map_err(|_| RuleError::Runtime(format!("duration `{spec}`: bad amount")))?;
    if n <= 0 {
        return Err(RuleError::Runtime(format!(
            "duration `{spec}`: must be positive"
        )));
    }
    let factor = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        other => {
            return Err(RuleError::Runtime(format!(
                "duration `{spec}`: unknown unit `{other}` (use s, m, h, d)"
            )))
        }
    };
    Ok(n * factor)
}
