//! An operator-declared schema blob for a datasource.
//!
//! Optional alternative (or supplement) to live `information_schema`
//! introspection (docs "Schema discovery"): an operator may declare the tables
//! and columns a datasource exposes directly in `datasources.json`, so the
//! authoring and AI surfaces have something to show even when introspection is
//! restricted or the read-only role cannot see the catalog.

use serde::{Deserialize, Serialize};

/// A declared schema: the tables a datasource exposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaBlob {
    pub tables: Vec<TableSchema>,
}

/// A declared table and its columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

/// A declared column name and engine type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub type_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let blob = SchemaBlob {
            tables: vec![TableSchema {
                name: "readings".into(),
                columns: vec![ColumnSchema {
                    name: "ts".into(),
                    type_name: "timestamptz".into(),
                }],
            }],
        };
        let json = serde_json::to_string(&blob).unwrap();
        let back: SchemaBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(blob, back);
    }
}
