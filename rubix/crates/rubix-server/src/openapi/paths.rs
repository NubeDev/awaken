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
    BatchQueryRequest, BatchQueryResponse, CreateDeviceRequest, CreatePrincipalRequest,
    CreateRecordRequest, CreateTenantRequest, CreatedPrincipalDto, DatasourceDto, DeviceDto,
    GrantDto, LoginRequest, LoginResponse, MeResponse, PreferencesDto, PrincipalDto, QueryRequest,
    QueryResponse, QuerySchemaResponse, RecordDto, RegisterDatasourceRequest, TenantDto,
    UpdateDatasourceRequest,
    UpdateDeviceRequest, UpdatePreferencesRequest, UpdatePrincipalRequest, UpdateRecordRequest,
};

/// `POST /auth/login`.
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "An opaque bearer token and its expiry", body = LoginResponse),
        (status = 401, description = "Unknown subject or wrong secret")
    )
)]
pub fn login() {}

/// `POST /auth/logout`.
#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 204, description = "Token revoked (idempotent)"),
        (status = 400, description = "No bearer token presented")
    )
)]
pub fn logout() {}

/// `GET /auth/me`.
#[utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "The current principal and its grants", body = MeResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub fn me() {}

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
    params(
        ("kind" = Option<String>, Query, description = "Filter to one collection (content.kind)"),
        ("tag" = Option<String>, Query, description = "Comma-separated tag names the record must all carry")
    ),
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
        (status = 403, description = "Principal lacks the write capability"),
        (status = 422, description = "Content failed its collection contract")
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

/// `POST /query/batch`.
#[utoipa::path(
    post,
    path = "/query/batch",
    request_body = BatchQueryRequest,
    responses(
        (status = 200, description = "Per-item query results (one bad item does not fail the batch)", body = BatchQueryResponse),
        (status = 400, description = "Empty batch or too many queries"),
        (status = 403, description = "Principal lacks the external-query capability")
    )
)]
pub fn run_batch() {}

/// `GET /query/schema`.
#[utoipa::path(
    get,
    path = "/query/schema",
    responses(
        (status = 200, description = "Readable tables + columns for the principal", body = QuerySchemaResponse),
        (status = 403, description = "Principal lacks the external-query capability")
    )
)]
pub fn query_schema() {}

/// `GET /prefs`.
#[utoipa::path(
    get,
    path = "/prefs",
    responses((status = 200, description = "The principal's display preferences", body = PreferencesDto))
)]
pub fn get_prefs() {}

/// `PATCH /prefs`.
#[utoipa::path(
    patch,
    path = "/prefs",
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "The updated preferences", body = PreferencesDto),
        (status = 400, description = "An unknown unit system")
    )
)]
pub fn patch_prefs() {}

/// `GET /datasources`.
#[utoipa::path(
    get,
    path = "/datasources",
    responses((status = 200, description = "Declared datasources", body = [DatasourceDto]))
)]
pub fn list_datasources() {}

/// `POST /datasources`.
#[utoipa::path(
    post,
    path = "/datasources",
    request_body = RegisterDatasourceRequest,
    responses(
        (status = 200, description = "Registered datasource", body = DatasourceDto),
        (status = 400, description = "Unsupported connector kind"),
        (status = 403, description = "Principal lacks the datasource-register capability"),
        (status = 409, description = "A datasource is already registered under this id"),
        (status = 502, description = "The connector could not reach its backend")
    )
)]
pub fn create_datasource() {}

/// `GET /datasources/{id}`.
#[utoipa::path(
    get,
    path = "/datasources/{id}",
    params(("id" = String, Path, description = "Datasource id")),
    responses(
        (status = 200, description = "The declared datasource", body = DatasourceDto),
        (status = 404, description = "No datasource registered under this id")
    )
)]
pub fn get_datasource() {}

/// `PATCH /datasources/{id}`.
#[utoipa::path(
    patch,
    path = "/datasources/{id}",
    params(("id" = String, Path, description = "Datasource id")),
    request_body = UpdateDatasourceRequest,
    responses(
        (status = 200, description = "Updated datasource", body = DatasourceDto),
        (status = 403, description = "Principal lacks the datasource-register capability"),
        (status = 404, description = "No datasource registered under this id"),
        (status = 502, description = "The connector could not reach its backend")
    )
)]
pub fn update_datasource() {}

/// `DELETE /datasources/{id}`.
#[utoipa::path(
    delete,
    path = "/datasources/{id}",
    params(("id" = String, Path, description = "Datasource id")),
    responses(
        (status = 204, description = "Datasource deregistered"),
        (status = 403, description = "Principal lacks the capability, or the native id cannot be removed"),
        (status = 404, description = "No datasource registered under this id")
    )
)]
pub fn delete_datasource() {}

/// `GET /ws/records`.
#[utoipa::path(
    get,
    path = "/ws/records",
    responses((status = 101, description = "WebSocket upgrade to the live-query feed"))
)]
pub fn subscribe_records() {}

// --- Admin & management surface (ADMIN-API.md) ---

/// `POST /principals`.
#[utoipa::path(
    post,
    path = "/principals",
    request_body = CreatePrincipalRequest,
    responses(
        (status = 201, description = "The created principal (with a generated secret if none was supplied)", body = CreatedPrincipalDto),
        (status = 400, description = "Unknown kind or role"),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 409, description = "A principal already exists under this subject")
    )
)]
pub fn create_principal() {}

/// `GET /principals`.
#[utoipa::path(
    get,
    path = "/principals",
    responses(
        (status = 200, description = "Principals in the caller's namespace", body = [PrincipalDto]),
        (status = 403, description = "Caller is not an admin in this namespace")
    )
)]
pub fn list_principals() {}

/// `GET /principals/{subject}`.
#[utoipa::path(
    get,
    path = "/principals/{subject}",
    params(("subject" = String, Path, description = "API-local principal subject")),
    responses(
        (status = 200, description = "The principal", body = PrincipalDto),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 404, description = "No such principal in this namespace")
    )
)]
pub fn get_principal() {}

/// `PATCH /principals/{subject}`.
#[utoipa::path(
    patch,
    path = "/principals/{subject}",
    params(("subject" = String, Path, description = "API-local principal subject")),
    request_body = UpdatePrincipalRequest,
    responses(
        (status = 200, description = "The updated principal", body = PrincipalDto),
        (status = 400, description = "Unknown role"),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 404, description = "No such principal in this namespace"),
        (status = 409, description = "Refused: would remove the last admin")
    )
)]
pub fn update_principal() {}

/// `DELETE /principals/{subject}`.
#[utoipa::path(
    delete,
    path = "/principals/{subject}",
    params(("subject" = String, Path, description = "API-local principal subject")),
    responses(
        (status = 204, description = "Principal deleted"),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 404, description = "No such principal in this namespace"),
        (status = 409, description = "Refused: would remove the last admin")
    )
)]
pub fn delete_principal() {}

/// `GET /principals/{subject}/grants`.
#[utoipa::path(
    get,
    path = "/principals/{subject}/grants",
    params(("subject" = String, Path, description = "API-local principal subject")),
    responses(
        (status = 200, description = "The principal's capability grants", body = [GrantDto]),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 404, description = "No such principal in this namespace")
    )
)]
pub fn list_grants() {}

/// `PUT /principals/{subject}/grants/{capability}`.
#[utoipa::path(
    put,
    path = "/principals/{subject}/grants/{capability}",
    params(
        ("subject" = String, Path, description = "API-local principal subject"),
        ("capability" = String, Path, description = "Capability wire string")
    ),
    responses(
        (status = 200, description = "The grant (idempotent)", body = GrantDto),
        (status = 400, description = "Unknown capability"),
        (status = 403, description = "Caller may not administer this grant"),
        (status = 404, description = "No such principal in this namespace")
    )
)]
pub fn put_grant() {}

/// `DELETE /principals/{subject}/grants/{capability}`.
#[utoipa::path(
    delete,
    path = "/principals/{subject}/grants/{capability}",
    params(
        ("subject" = String, Path, description = "API-local principal subject"),
        ("capability" = String, Path, description = "Capability wire string")
    ),
    responses(
        (status = 204, description = "Grant revoked (idempotent)"),
        (status = 400, description = "Unknown capability"),
        (status = 403, description = "Caller may not administer this grant"),
        (status = 404, description = "No such principal in this namespace")
    )
)]
pub fn delete_grant() {}

/// `POST /tenants`.
#[utoipa::path(
    post,
    path = "/tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "The onboarded tenant", body = TenantDto),
        (status = 400, description = "Invalid tenant id"),
        (status = 403, description = "Caller is not a root/system principal"),
        (status = 409, description = "Edge node, or a tenant already exists under this id")
    )
)]
pub fn create_tenant() {}

/// `GET /tenants`.
#[utoipa::path(
    get,
    path = "/tenants",
    responses(
        (status = 200, description = "Onboarded tenants (cloud) or the single namespace (edge)", body = [TenantDto]),
        (status = 403, description = "Caller is not a root/system principal")
    )
)]
pub fn list_tenants() {}

/// `DELETE /tenants/{id}`.
#[utoipa::path(
    delete,
    path = "/tenants/{id}",
    params(
        ("id" = String, Path, description = "Tenant id"),
        ("confirm" = Option<String>, Query, description = "Must echo the tenant id to confirm the irreversible delete")
    ),
    responses(
        (status = 204, description = "Tenant namespace purged and deregistered"),
        (status = 400, description = "Missing or mismatched confirmation"),
        (status = 403, description = "Caller is not a root/system principal"),
        (status = 404, description = "No tenant onboarded under this id"),
        (status = 409, description = "Edge node: tenant deletion is a cloud action")
    )
)]
pub fn delete_tenant() {}

/// `POST /devices`.
#[utoipa::path(
    post,
    path = "/devices",
    request_body = CreateDeviceRequest,
    responses(
        (status = 201, description = "The registered device", body = DeviceDto),
        (status = 403, description = "Caller is not an admin / lacks device-manage"),
        (status = 409, description = "A device already exists under this id")
    )
)]
pub fn create_device() {}

/// `GET /devices`.
#[utoipa::path(
    get,
    path = "/devices",
    responses(
        (status = 200, description = "Devices in the caller's namespace", body = [DeviceDto]),
        (status = 403, description = "Caller is not an admin in this namespace")
    )
)]
pub fn list_devices() {}

/// `GET /devices/{id}`.
#[utoipa::path(
    get,
    path = "/devices/{id}",
    params(("id" = String, Path, description = "API-local device id")),
    responses(
        (status = 200, description = "The device", body = DeviceDto),
        (status = 403, description = "Caller is not an admin in this namespace"),
        (status = 404, description = "No such device in this namespace")
    )
)]
pub fn get_device() {}

/// `PATCH /devices/{id}`.
#[utoipa::path(
    patch,
    path = "/devices/{id}",
    params(("id" = String, Path, description = "API-local device id")),
    request_body = UpdateDeviceRequest,
    responses(
        (status = 200, description = "The updated device", body = DeviceDto),
        (status = 403, description = "Caller is not an admin / lacks device-manage"),
        (status = 404, description = "No such device in this namespace")
    )
)]
pub fn update_device() {}

/// `DELETE /devices/{id}`.
#[utoipa::path(
    delete,
    path = "/devices/{id}",
    params(("id" = String, Path, description = "API-local device id")),
    responses(
        (status = 204, description = "Device deregistered"),
        (status = 403, description = "Caller is not an admin / lacks device-manage"),
        (status = 404, description = "No such device in this namespace")
    )
)]
pub fn delete_device() {}
