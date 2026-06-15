//! The OpenAPI 3.1 document describing the transport surface.
//!
//! Served at `/api-docs/openapi.json` (`rubix/docs/sessions/WS-16.md`). The
//! document is derived with utoipa from the route set and DTO schemas, so it
//! stays in step with the handlers it describes. Paths are declared here (rather
//! than per-handler macros) to keep the handlers thin extract→call→map bodies and
//! gather the surface in one readable place.

use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiDoc;

use crate::dto::{
    CreateRecordRequest, DatasourceDto, QueryRequest, QueryResponse, RecordDto,
    RegisterDatasourceRequest, UpdateDatasourceRequest, UpdateRecordRequest,
};

/// The OpenAPI definition for the rubix transport.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rubix Platform API",
        description = "Edge-to-cloud data platform transport: records, query, datasources, realtime."
    ),
    paths(
        crate::openapi::paths::health,
        crate::openapi::paths::list_records,
        crate::openapi::paths::create_record,
        crate::openapi::paths::get_record,
        crate::openapi::paths::update_record,
        crate::openapi::paths::delete_record,
        crate::openapi::paths::run_query,
        crate::openapi::paths::list_datasources,
        crate::openapi::paths::create_datasource,
        crate::openapi::paths::get_datasource,
        crate::openapi::paths::update_datasource,
        crate::openapi::paths::delete_datasource,
        crate::openapi::paths::subscribe_records
    ),
    components(schemas(
        RecordDto,
        CreateRecordRequest,
        UpdateRecordRequest,
        QueryRequest,
        QueryResponse,
        DatasourceDto,
        RegisterDatasourceRequest,
        UpdateDatasourceRequest
    ))
)]
pub struct ApiDoc;

/// Build the OpenAPI document.
#[must_use]
pub fn document() -> OpenApiDoc {
    ApiDoc::openapi()
}
