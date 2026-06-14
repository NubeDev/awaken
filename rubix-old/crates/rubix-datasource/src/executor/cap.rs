//! Apply a datasource's [`Caps`] to a backend's raw rows.
//!
//! The breach semantics are deliberately surfaced, not decided here (docs
//! "Caps-breach semantics"): this turns a [`RawResult`] into a [`ResultSet`]
//! whose `breached` flag the caller inspects. The lenient (dashboard) path
//! reads the truncated `ResultSet`; the strict (spark) path calls
//! [`into_strict`] to turn a breach into [`DatasourceError::CapBreached`].

use crate::backend::{RawResult, ResultSet};
use crate::caps::{CapState, Caps};
use crate::error::{DatasourceError, DatasourceResult};

/// Byte cost of one row: the serialized length of its JSON cells. Approximate
/// (matches how the row travels to a browser), used only for the byte cap.
fn row_bytes(row: &[serde_json::Value]) -> u64 {
    row.iter()
        .map(|v| v.to_string().len() as u64)
        .sum::<u64>()
}

/// Collect rows from `raw` until a cap would be breached, returning the
/// truncated [`ResultSet`] with `breached` set if a row was dropped. Wall-clock
/// is enforced by the backend, so only row/byte axes apply here.
pub fn apply(raw: RawResult, caps: &Caps) -> ResultSet {
    let mut state = CapState::default();
    let mut rows = Vec::new();
    for row in raw.rows {
        if state.admit(row_bytes(&row), caps) {
            rows.push(row);
        } else {
            break;
        }
    }
    ResultSet {
        columns: raw.columns,
        rows,
        breached: state.breached,
    }
}

/// The strict (spark) path: a breach is an error carrying what was collected,
/// not a truncated finding (docs "Truncation on the spark path").
pub fn into_strict(datasource: &str, result: ResultSet) -> DatasourceResult<ResultSet> {
    if result.breached {
        return Err(DatasourceError::CapBreached {
            datasource: datasource.to_string(),
            rows: result.rows.len() as u64,
            bytes: result.rows.iter().map(|r| row_bytes(r)).sum(),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Column;
    use serde_json::json;

    fn raw(n: usize) -> RawResult {
        RawResult {
            columns: vec![Column {
                name: "n".into(),
                type_name: "json".into(),
            }],
            rows: (0..n).map(|i| vec![json!(i)]).collect(),
        }
    }

    #[test]
    fn truncates_at_row_cap_and_flags_breach() {
        let rs = apply(raw(5), &Caps::rows(3));
        assert_eq!(rs.rows.len(), 3);
        assert!(rs.breached);
    }

    #[test]
    fn under_cap_is_not_breached() {
        let rs = apply(raw(2), &Caps::rows(3));
        assert_eq!(rs.rows.len(), 2);
        assert!(!rs.breached);
    }

    #[test]
    fn strict_path_errors_on_breach() {
        let rs = apply(raw(5), &Caps::rows(3));
        let err = into_strict("h", rs).unwrap_err();
        assert!(matches!(err, DatasourceError::CapBreached { rows: 3, .. }));
    }

    #[test]
    fn strict_path_passes_clean_result() {
        let rs = apply(raw(2), &Caps::rows(3));
        assert!(into_strict("h", rs).is_ok());
    }
}
