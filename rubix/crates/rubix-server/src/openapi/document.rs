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
    AgentDto, AppendReadingsRequest, AppendReadingsResponse, AskRequest, AskResponse,
    BatchQueryItem, BatchQueryRequest, BatchQueryResponse, BatchQueryResult, BindingDto, BucketDto,
    CatalogResponse, ColumnDto, CreateDeviceRequest, CreatePrincipalRequest, CreateRecordRequest,
    CreateRuleRequest, CreateTenantRequest, CreatedPrincipalDto, DatasourceDto, DeviceDto,
    DryRunRequest, DryRunResponse, FileRefDto, FilterFacetDto, GrantDto, LoginRequest, LoginResponse,
    MeResponse, PersistRequest,
    PersistedDto, PreferencesDto, PrincipalDto, ProvisionAgentRequest, ProvisionedAgentDto,
    QueryRequest, QueryResponse, QuerySchemaResponse, QueryVariableDto, ReadingDto, ReadingSampleDto,
    RecallRequest,
    RecalledDto, RecordDto, RegisterDatasourceRequest, ResolvedInputDto, RuleDto, TableSchemaDto,
    TenantDto, TimeBoundDto, TimeScopeDto, TransformDto, UpdateDatasourceRequest,
    UpdateDeviceRequest, UpdatePreferencesRequest, UpdatePrincipalRequest, UpdateRecordRequest,
    UpdateRuleRequest,
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
        crate::openapi::paths::read_readings,
        crate::openapi::paths::append_readings,
        crate::openapi::paths::get_record,
        crate::openapi::paths::update_record,
        crate::openapi::paths::delete_record,
        crate::openapi::paths::run_query,
        crate::openapi::paths::run_batch,
        crate::openapi::paths::query_schema,
        crate::openapi::paths::get_prefs,
        crate::openapi::paths::patch_prefs,
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
        crate::openapi::paths::delete_device,
        crate::openapi::paths::list_rules,
        crate::openapi::paths::create_rule,
        crate::openapi::paths::get_rule,
        crate::openapi::paths::update_rule,
        crate::openapi::paths::delete_rule,
        crate::openapi::paths::dryrun_rule,
        crate::openapi::paths::referencing_rules,
        crate::openapi::paths::rules_catalog,
        crate::openapi::paths::upload_file,
        crate::openapi::paths::download_file
    ),
    components(schemas(
        LoginRequest,
        LoginResponse,
        MeResponse,
        RecordDto,
        CreateRecordRequest,
        UpdateRecordRequest,
        AppendReadingsRequest,
        AppendReadingsResponse,
        ReadingSampleDto,
        ReadingDto,
        QueryRequest,
        QueryResponse,
        QuerySchemaResponse,
        TableSchemaDto,
        ColumnDto,
        TimeScopeDto,
        TimeBoundDto,
        TransformDto,
        QueryVariableDto,
        BatchQueryRequest,
        BatchQueryItem,
        BatchQueryResponse,
        BatchQueryResult,
        PreferencesDto,
        UpdatePreferencesRequest,
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
        AskResponse,
        RuleDto,
        BindingDto,
        CreateRuleRequest,
        UpdateRuleRequest,
        DryRunRequest,
        DryRunResponse,
        CatalogResponse,
        FilterFacetDto,
        ResolvedInputDto,
        BucketDto,
        FileRefDto
    ))
)]
pub struct ApiDoc;

/// Build the OpenAPI document.
#[must_use]
pub fn document() -> OpenApiDoc {
    ApiDoc::openapi()
}
