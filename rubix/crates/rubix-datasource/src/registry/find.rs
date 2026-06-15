//! Look up one declared datasource's identity by id.
//!
//! The single-datasource read behind a control-plane `GET /datasources/:id`
//! (`rubix/docs/SCOPE.md`, "Datasources"). Like [`list`](super::list::list) it is
//! an open read over declared identity — it reveals only a name/label/kind, which
//! the query gate already governs. An unknown id fails closed with
//! [`DatasourceError::Unknown`], matching [`resolve`](super::resolve::resolve).

use crate::connector::DatasourceConfig;
use crate::error::{DatasourceError, Result};

use super::store::Registry;

/// The declared identity of the datasource registered under `id`.
///
/// # Errors
/// Returns [`DatasourceError::Unknown`] if no datasource is registered under `id`.
pub fn find<'a>(registry: &'a Registry, id: &str) -> Result<&'a DatasourceConfig> {
    registry
        .get(id)
        .map(|entry| entry.config())
        .ok_or_else(|| DatasourceError::Unknown(id.to_owned()))
}
