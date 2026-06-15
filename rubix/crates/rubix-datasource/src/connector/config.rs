//! The declared identity of a datasource.
//!
//! A datasource is *declared* before it is read (`rubix/docs/SCOPE.md`,
//! "Datasources"): the declaration is its stable id and a human label. The
//! connector-specific wiring (a Postgres connection string, a SurrealDB session)
//! lives in each connector impl, not here, so this config stays the common
//! identity every connector shares and the registry keys on.

/// The kind of the native SurrealDB datasource.
///
/// External connectors carry their own kind (`"postgres"`, …); the native default
/// uses this. Kept here so [`DatasourceConfig`] has a single source of truth for
/// the kind a list/GET reports.
pub const NATIVE_KIND: &str = "surrealdb";

/// The stable, connector-agnostic identity of a datasource.
///
/// `id` is the key the registry stores under and a query refers to; `label` is a
/// human-readable name for dashboards; `kind` is the connector family (`"postgres"`,
/// the native `"surrealdb"`, …) a control-plane GET reports and persistence keys
/// rehydration on. Two datasources never share an id — the registry refuses a
/// duplicate (`super::super::registry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasourceConfig {
    id: String,
    label: String,
    kind: String,
}

impl DatasourceConfig {
    /// Declare a datasource with a stable `id` and a human `label`, defaulting to
    /// the native [`NATIVE_KIND`]. External connectors override the kind with
    /// [`with_kind`](Self::with_kind).
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: NATIVE_KIND.to_owned(),
        }
    }

    /// Set the connector family this datasource belongs to (`"postgres"`, …).
    #[must_use]
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
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

    /// The connector family (`"postgres"`, the native `"surrealdb"`, …).
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
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

    #[test]
    fn config_defaults_to_the_native_kind_and_honors_an_override() {
        let native = DatasourceConfig::new("surrealdb", "SurrealDB");
        assert_eq!(native.kind(), super::NATIVE_KIND);
        let pg = DatasourceConfig::new("warehouse", "Cloud Warehouse").with_kind("postgres");
        assert_eq!(pg.kind(), "postgres");
    }
}
