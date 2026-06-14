//! The in-memory map of datasource id -> entry, seeded with the native default.
//!
//! The registry is the lookup the unified query surface reads from
//! (`rubix/docs/SCOPE.md`, "Datasources"). It is created with the native SurrealDB
//! datasource already present as the default entry; further connectors are added
//! through [`register`](super::register). Lookup and the spanning query read it
//! immutably.

use std::collections::HashMap;

use crate::connector::DatasourceConfig;

use super::entry::DatasourceEntry;

/// The reserved id of the native SurrealDB datasource, present in every registry.
///
/// SurrealDB is the native/default datasource (`rubix/docs/SCOPE.md`); its tables
/// are the canonical ones the query surface already scans through the scoped
/// session. The id is reserved so an external connector cannot shadow it.
pub const NATIVE_SURREAL_ID: &str = "surrealdb";

/// The set of declared datasources, keyed by stable id.
///
/// Constructed with the native SurrealDB entry already registered (the default,
/// `rubix/docs/SCOPE.md`). External connectors are added with
/// [`register`](super::register::register), looked up with
/// [`resolve`](super::resolve::resolve), and unioned with SurrealDB by
/// [`span`](super::span::span).
pub struct Registry {
    entries: HashMap<String, DatasourceEntry>,
}

impl Registry {
    /// A registry seeded with only the native SurrealDB datasource.
    #[must_use]
    pub fn with_native_default() -> Self {
        let mut entries = HashMap::new();
        entries.insert(
            NATIVE_SURREAL_ID.to_owned(),
            DatasourceEntry::Native {
                config: DatasourceConfig::new(NATIVE_SURREAL_ID, "SurrealDB (native)"),
            },
        );
        Self { entries }
    }

    /// Whether a datasource is registered under `id`.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// The number of registered datasources, including the native default.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty. It never is — the native default is always
    /// present — but the accessor is provided for clippy's `len`/`is_empty` pair.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow the entry registered under `id`, if any.
    pub(crate) fn get(&self, id: &str) -> Option<&DatasourceEntry> {
        self.entries.get(id)
    }

    /// Insert a fully-built entry under its declared id. Crate-internal: callers
    /// go through [`register`](super::register::register) so the capability check
    /// and duplicate check always run first.
    pub(crate) fn insert(&mut self, id: String, entry: DatasourceEntry) {
        self.entries.insert(id, entry);
    }

    /// Every registered entry, for the spanning query to walk.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&String, &DatasourceEntry)> {
        self.entries.iter()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::with_native_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{NATIVE_SURREAL_ID, Registry};

    #[test]
    fn a_fresh_registry_holds_only_the_native_default() {
        let registry = Registry::with_native_default();
        assert!(registry.contains(NATIVE_SURREAL_ID));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn an_unregistered_id_is_absent() {
        let registry = Registry::with_native_default();
        assert!(!registry.contains("warehouse"));
    }
}
