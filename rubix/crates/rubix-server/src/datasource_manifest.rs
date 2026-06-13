//! Boot-time loading of the external datasource manifest (`datasources.json`).
//!
//! A datasource is a declared, read-only connection to an external SQL database
//! (primarily TimescaleDB/Postgres) that rubix *reads out of* at query time —
//! the opposite direction from a protocol driver, and a separate manifest
//! (`docs/design/datasources.md` "Boundaries"). The core engine lives in
//! `rubix-datasource`; this module is the thin host seam that reads the manifest
//! at boot and builds the live [`DatasourceRegistry`] placed in
//! [`crate::AppState`].
//!
//! Loading is fail-closed on a malformed manifest (a bad file is an operator
//! error, not something to swallow), but a *missing* file is a valid "no
//! datasources" configuration — exactly like `drivers.json`.

use std::path::Path;
use std::sync::Arc;

use rubix_datasource::{DatasourceEntry, DatasourceRegistry};

/// Load `datasources.json` and build the live registry, or return `None` when no
/// datasource is configured.
///
/// Returns `None` when the file is absent or holds an empty array, so callers
/// can leave [`crate::AppState::datasources`] `None` and withhold the
/// datasource surfaces entirely. A parse error or a connection failure (a bad
/// credential / unreachable host) is surfaced as an error: pools are built
/// eagerly in [`DatasourceRegistry::register_all`] so a misconfiguration fails
/// at boot rather than on the first read.
pub async fn load(path: &Path) -> anyhow::Result<Option<Arc<DatasourceRegistry>>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("read datasource manifest {path:?}: {e}"))?;
    let entries: Vec<DatasourceEntry> = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("parse datasource manifest {path:?}: {e}"))?;
    if entries.is_empty() {
        return Ok(None);
    }
    let mut registry = DatasourceRegistry::new();
    registry.register_all(entries).await?;
    Ok(Some(Arc::new(registry)))
}
