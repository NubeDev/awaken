//! OpenAPI path declarations for the transport routes.
//!
//! utoipa builds the `paths` section of the document from `#[utoipa::path]`
//! annotations. Keeping them here (one annotated declaration per route) rather
//! than on the handlers keeps the handlers thin extract→call→map bodies
//! (`rubix/docs/sessions/WS-16.md`) and gathers the documented surface in one
//! place next to [`document`](super::document). These functions exist only to
//! carry the annotation; they are never called — the real handlers live under
//! `http/` and `ws/`.
//!
//! They are referenced by the `#[utoipa::path]` derive in
//! [`document`](super::document) at macro-expansion, not at runtime, so the
//! never-called lint is allowed for the whole module.
#![allow(dead_code)]

use crate::dto::{
    CreateRecordRequest, DatasourceDto, QueryRequest, QueryResponse, RecordDto,
    UpdateRecordRequest,
};

/// `GET /health`.
#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Process and store are live"))
)]
pub fn health() {}

/// `GET /records`.
#[utoipa::path(
    get,
    path = "/records",
    responses((status = 200, description = "Records visible to the principal", body = [RecordDto]))
)]
pub fn list_records() {}

/// `POST /records`.
#[utoipa::path(
    post,
    path = "/records",
    request_body = CreateRecordRequest,
    responses(
        (status = 200, description = "The created record", body = RecordDto),
        (status = 403, description = "Principal lacks the write capability")
    )
)]
pub fn create_record() {}

/// `GET /records/{id}`.
#[utoipa::path(
    get,
    path = "/records/{id}",
    params(("id" = String, Path, description = "Record id")),
    responses(
        (status = 200, description = "The record", body = RecordDto),
        (status = 404, description = "Not found or not visible")
    )
)]
pub fn get_record() {}

/// `PATCH /records/{id}`.
#[utoipa::path(
    patch,
    path = "/records/{id}",
    params(("id" = String, Path, description = "Record id")),
    request_body = UpdateRecordRequest,
    responses(
        (status = 200, description = "The updated record", body = RecordDto),
        (status = 403, description = "Principal lacks the write capability")
    )
)]
pub fn update_record() {}

/// `DELETE /records/{id}`.
#[utoipa::path(
    delete,
    path = "/records/{id}",
    params(("id" = String, Path, description = "Record id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Principal lacks the write capability")
    )
)]
pub fn delete_record() {}

/// `POST /query`.
#[utoipa::path(
    post,
    path = "/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result rows", body = QueryResponse),
        (status = 403, description = "Principal lacks the external-query capability")
    )
)]
pub fn run_query() {}

/// `GET /datasources`.
#[utoipa::path(
    get,
    path = "/datasources",
    responses((status = 200, description = "Declared datasources", body = [DatasourceDto]))
)]
pub fn list_datasources() {}

/// `GET /ws/records`.
#[utoipa::path(
    get,
    path = "/ws/records",
    responses((status = 101, description = "WebSocket upgrade to the live-query feed"))
)]
pub fn subscribe_records() {}
