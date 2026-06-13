//! The returned row shape: `{ columns, rows }`, identical to what `rubix-query`
//! returns so every consumer renders or folds it the same way (docs "Model").
//!
//! Rows are `serde_json` values, schema inferred from the columns the engine
//! reports — the to_jsonb / Arrow-free path the prompt specifies. Each row is a
//! flat list of cell values positional against `columns`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A column reported by the external engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    /// The column name as the engine returned it.
    pub name: String,
    /// The engine's type name (e.g. `int8`, `timestamptz`), informational only.
    /// The crate does not map types — rows are JSON, schema inferred upstream.
    pub type_name: String,
}

/// One result row: a cell value per column, in column order.
pub type Row = Vec<Value>;

/// A complete read result: the columns and the rows that fit under the caps,
/// plus whether the read breached a cap. Whether `breached` is tolerated
/// (truncated view) or fatal is the caller's policy, not decided here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// True if a row was dropped because a cap was reached. On the dashboard
    /// path this is a truncated view; on the spark path the executor turns it
    /// into [`crate::DatasourceError::CapBreached`] (docs "Refresh cost").
    pub breached: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn result_set_serializes_columns_and_rows() {
        let rs = ResultSet {
            columns: vec![Column {
                name: "n".into(),
                type_name: "int8".into(),
            }],
            rows: vec![vec![json!(1)], vec![json!(2)]],
            breached: false,
        };
        let v = serde_json::to_value(&rs).unwrap();
        assert_eq!(v["columns"][0]["name"], "n");
        assert_eq!(v["rows"][1][0], 2);
        assert_eq!(v["breached"], false);
    }
}
