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
    AgentDto, AskRequest, AskResponse, CreateDeviceRequest, CreatePrincipalRequest,
    CreateRecordRequest, CreateTenantRequest, CreatedPrincipalDto, DatasourceDto, DeviceDto,
    GrantDto, LoginRequest, LoginResponse, MeResponse, PersistRequest, PersistedDto, PrincipalDto,
    ProvisionAgentRequest, ProvisionedAgentDto, QueryRequest, QueryResponse, RecallRequest,
    RecalledDto, RecordDto, RegisterDatasourceRequest, TenantDto, UpdateDatasourceRequest,
    UpdateDeviceRequest, UpdatePrincipalRequest, UpdateRecordRequest,
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
        crate::openapi::paths::login,
        crate::openapi::paths::logout,
        crate::openapi::paths::me,
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
        crate::openapi::paths::subscribe_records,
        crate::openapi::paths::create_principal,
        crate::openapi::paths::list_principals,
        crate::openapi::paths::get_principal,
        crate::openapi::paths::update_principal,
        crate::openapi::paths::delete_principal,
        crate::openapi::paths::list_grants,
        crate::openapi::paths::put_grant,
        crate::openapi::paths::delete_grant,
        crate::openapi::paths::create_tenant,
        crate::openapi::paths::list_tenants,
        crate::openapi::paths::delete_tenant,
        crate::openapi::paths::create_device,
        crate::openapi::paths::list_devices,
        crate::openapi::paths::get_device,
        crate::openapi::paths::update_device,
        crate::openapi::paths::delete_device
    ),
    components(schemas(
        LoginRequest,
        LoginResponse,
        MeResponse,
        RecordDto,
        CreateRecordRequest,
        UpdateRecordRequest,
        QueryRequest,
        QueryResponse,
        DatasourceDto,
        RegisterDatasourceRequest,
        UpdateDatasourceRequest,
        PrincipalDto,
        CreatePrincipalRequest,
        CreatedPrincipalDto,
        UpdatePrincipalRequest,
        GrantDto,
        TenantDto,
        CreateTenantRequest,
        DeviceDto,
        CreateDeviceRequest,
        UpdateDeviceRequest,
        AgentDto,
        ProvisionAgentRequest,
        ProvisionedAgentDto,
        RecallRequest,
        RecalledDto,
        PersistRequest,
        PersistedDto,
        AskRequest,
        AskResponse
    ))
)]
pub struct ApiDoc;

/// Build the OpenAPI document.
#[must_use]
pub fn document() -> OpenApiDoc {
    ApiDoc::openapi()
}
