//! Wire shapes for the datasource registry.
//!
//! The transport DTO for `GET /datasources` (`rubix/docs/sessions/WS-16.md`): the
//! declared identity of each registered datasource a dashboard can pick from
//! (`rubix/docs/SCOPE.md`, "Datasources"). Listing is an open read — it reveals
//! only names, which the query capability already governs.

use serde::Serialize;
use utoipa::ToSchema;

/// One declared datasource's identity.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DatasourceDto {
    /// The datasource's stable id (the catalog schema queries address it by).
    pub id: String,
    /// The human-facing label.
    pub label: String,
}
