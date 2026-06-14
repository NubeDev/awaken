//! The declared identity of a datasource.
//!
//! A datasource is *declared* before it is read (`rubix/docs/SCOPE.md`,
//! "Datasources"): the declaration is its stable id and a human label. The
//! connector-specific wiring (a Postgres connection string, a SurrealDB session)
//! lives in each connector impl, not here, so this config stays the common
//! identity every connector shares and the registry keys on.

/// The stable, connector-agnostic identity of a datasource.
///
/// `id` is the key the registry stores under and a query refers to; `label` is a
/// human-readable name for dashboards. Two datasources never share an id — the
/// registry refuses a duplicate (`super::super::registry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasourceConfig {
    id: String,
    label: String,
}

impl DatasourceConfig {
    /// Declare a datasource with a stable `id` and a human `label`.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }

    /// The stable id the registry keys on and a query refers to.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The human-readable label for dashboards.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::DatasourceConfig;

    #[test]
    fn config_exposes_its_id_and_label() {
        let cfg = DatasourceConfig::new("warehouse", "Cloud Warehouse");
        assert_eq!(cfg.id(), "warehouse");
        assert_eq!(cfg.label(), "Cloud Warehouse");
    }
}
