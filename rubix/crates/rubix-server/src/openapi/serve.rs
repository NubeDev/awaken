//! `GET /api-docs/openapi.json` — serve the OpenAPI document.
//!
//! Exposes the utoipa-built OpenAPI 3.1 document (`rubix/docs/sessions/
//! WS-16.md`). The document is built once per request from the static definition;
//! it lists every registered route so a client (or codegen) can discover the
//! surface.

use axum::Json;

use super::document::document;

/// Return the OpenAPI document as JSON.
pub async fn serve_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(document())
}
