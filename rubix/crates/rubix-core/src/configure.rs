//! Runtime configuration: which SurrealDB engine, which namespace/database, and
//! which deployment profile.
//!
//! Edge is the default profile (single namespace); cloud selects
//! namespace-per-tenant. The file-backed default engine is SurrealKV; tests use
//! the in-memory engine (`rubix/STACK-DEISGN.md`, "Key decisions").

use serde::{Deserialize, Serialize};

/// The embedded SurrealDB storage engine to open.
///
/// Contract #6: SurrealDB is the one engine. The choice here is only the
/// embedded backend, not a second store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreEngine {
    /// In-memory engine — non-durable, used for tests.
    Memory,
    /// File-backed SurrealKV engine rooted at the given path — the durable
    /// default for a running node.
    File {
        /// Filesystem path for the SurrealKV data directory.
        path: String,
    },
}

/// Deployment profile.
///
/// Edge resolves to a single tenant automatically; cloud is
/// namespace-per-tenant (`rubix/docs/SCOPE.md`, "Edge and cloud profiles").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Profile {
    /// Single-namespace edge node.
    #[default]
    Edge,
    /// Multi-tenant cloud deployment.
    Cloud,
}

/// The configuration the store and server are built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Which embedded engine to open.
    pub engine: StoreEngine,
    /// SurrealDB namespace to bootstrap and use.
    pub namespace: String,
    /// SurrealDB database to bootstrap and use.
    pub database: String,
    /// Deployment profile.
    pub profile: Profile,
}

impl RuntimeConfig {
    /// An in-memory configuration for tests and ephemeral runs.
    #[must_use]
    pub fn in_memory(namespace: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            engine: StoreEngine::Memory,
            namespace: namespace.into(),
            database: database.into(),
            profile: Profile::Edge,
        }
    }

    /// A file-backed edge configuration rooted at `path`.
    #[must_use]
    pub fn file_backed(
        path: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            engine: StoreEngine::File { path: path.into() },
            namespace: namespace.into(),
            database: database.into(),
            profile: Profile::Edge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Profile, RuntimeConfig, StoreEngine};

    #[test]
    fn in_memory_selects_the_memory_engine() {
        let cfg = RuntimeConfig::in_memory("rubix", "main");
        assert_eq!(cfg.engine, StoreEngine::Memory);
        assert_eq!(cfg.profile, Profile::Edge);
        assert_eq!(cfg.namespace, "rubix");
        assert_eq!(cfg.database, "main");
    }

    #[test]
    fn file_backed_selects_the_file_engine_with_path() {
        let cfg = RuntimeConfig::file_backed("/var/lib/rubix", "rubix", "main");
        assert_eq!(
            cfg.engine,
            StoreEngine::File {
                path: "/var/lib/rubix".into()
            }
        );
    }

    #[test]
    fn profile_defaults_to_edge() {
        assert_eq!(Profile::default(), Profile::Edge);
    }
}
