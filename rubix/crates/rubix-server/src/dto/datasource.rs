//! Wire shapes for the datasource registry.
//!
//! The transport DTOs for the `/datasources` control plane (`rubix/docs/sessions/
//! WS-16.md`): the declared identity of each registered datasource a dashboard can
//! pick from (`rubix/docs/SCOPE.md`, "Datasources"), plus the register/update
//! request bodies. Listing/reading is an open read — it reveals only
//! id/label/kind, never a connection secret. Registering crosses the WS-04
//! `datasource-register` capability.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One declared datasource's identity.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DatasourceDto {
    /// The datasource's stable id (the catalog schema queries address it by).
    pub id: String,
    /// The human-facing label.
    pub label: String,
    /// The connector family (`"postgres"`, the native `"surrealdb"`, …).
    pub kind: String,
}

/// The body of `POST /datasources` — register a new external connector.
///
/// `connection_string` is the connector's wiring (a `postgres://` URL); it is
/// stored to rehydrate the connector on restart and is never echoed back on the
/// wire. `tables` is the set of table names the connector exposes to the query
/// surface.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterDatasourceRequest {
    /// The stable id to register under. Must be unique and not the native id.
    pub id: String,
    /// The human-facing label.
    pub label: String,
    /// The connector family. Only `"postgres"` is supported (feature-gated).
    pub kind: String,
    /// The connector's connection string (e.g. a `postgres://` URL).
    pub connection_string: String,
    /// The table names the connector offers to the unified query surface.
    pub tables: Vec<String>,
}

/// The body of `PATCH /datasources/:id` — update a registered connector.
///
/// An update re-registers the connector: the label and the connection/tables can
/// change, so the providers are rebuilt. The id in the path is authoritative; a
/// body `id` is ignored.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDatasourceRequest {
    /// A new human-facing label, if changing it.
    pub label: Option<String>,
    /// A new connection string, if re-pointing the connector.
    pub connection_string: Option<String>,
    /// A new table set, if changing what the connector exposes.
    pub tables: Option<Vec<String>>,
}
