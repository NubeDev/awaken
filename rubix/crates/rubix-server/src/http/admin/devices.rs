//! `/devices` — the edge device registry, gate-written and capability-guarded.
//!
//! Surface 4 of `rubix/docs/design/ADMIN-API.md`. A device is a control-plane
//! *registration* (who is a device, its label/class/metadata), distinct from
//! commanding the hardware (`DeviceActuate`). It is a gate-written record: the
//! discriminator `content.kind == "device"` puts it in the device collection, and
//! the record id is namespace-prefixed (`{namespace}_{id}`) for the same
//! per-tenant isolation as principals. Mutations cross the gate as a `Command`
//! gated by the new `DeviceManage` capability — so each create/update/delete is
//! audited with a correlation id. Reads run on the scoped session, filtered to the
//! device collection.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_core::{Id, Record, read_record};
use rubix_gate::{Capability, Change, Command, apply, read_record_on_session, read_records_on_session_filtered};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::auth::Authenticated;
use crate::dto::admin::{CreateDeviceRequest, DeviceDto, UpdateDeviceRequest};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::guard::require_admin;

/// The collection discriminator that marks a record as a device registry entry.
const DEVICE_KIND: &str = "device";
/// The capability governing device-registry mutations.
const DEVICE_MANAGE: Capability = Capability::DeviceManage;

/// `POST /devices` — register a new device (Admin + `device-manage`).
pub async fn create_device_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreateDeviceRequest>,
) -> ApiResult<(StatusCode, Json<DeviceDto>)> {
    let namespace = require_admin(&auth.principal)?;
    let storage_id = device_storage_id(&namespace, &body.id);

    // Caller-supplied id is unique within the namespace; a collision is `409`.
    if read_record(state.store.raw(), &Id::from_raw(storage_id.clone()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "a device is already registered under id `{}`",
            body.id
        )));
    }

    let content = device_content(&body.label, &body.kind, &body.metadata);
    let command = Command::new(
        auth.principal.clone(),
        DEVICE_MANAGE,
        Id::from_raw(storage_id.clone()),
        Change::Create(content),
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_device_error)?;

    let stored = read_record(state.store.raw(), &Id::from_raw(storage_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    Ok((StatusCode::CREATED, Json(device_dto(&stored, &namespace)?)))
}

/// `GET /devices` — list every device in the caller's namespace.
pub async fn list_devices_route(
    State(_state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<DeviceDto>>> {
    let namespace = require_admin(&auth.principal)?;
    let records = read_records_on_session_filtered(&auth.session, Some(DEVICE_KIND), &[])
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let devices = records
        .iter()
        .map(|r| device_dto(r, &namespace))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(devices))
}

/// `GET /devices/:id` — fetch one device, or `404`.
pub async fn get_device_route(
    State(_state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<Json<DeviceDto>> {
    let namespace = require_admin(&auth.principal)?;
    let record = load_device(&auth, &namespace, &id).await?;
    Ok(Json(device_dto(&record, &namespace)?))
}

/// `PATCH /devices/:id` — update a device (Admin + `device-manage`).
pub async fn update_device_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
    Json(body): Json<UpdateDeviceRequest>,
) -> ApiResult<Json<DeviceDto>> {
    let namespace = require_admin(&auth.principal)?;
    let current = load_device(&auth, &namespace, &id).await?;
    let current = device_dto(&current, &namespace)?;

    let label = body.label.unwrap_or(current.label);
    let kind = body.kind.unwrap_or(current.kind);
    let metadata = body.metadata.unwrap_or(current.metadata);

    let storage_id = device_storage_id(&namespace, &id);
    let command = Command::new(
        auth.principal.clone(),
        DEVICE_MANAGE,
        Id::from_raw(storage_id.clone()),
        Change::Update(device_content(&label, &kind, &metadata)),
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_device_error)?;

    let stored = read_record(state.store.raw(), &Id::from_raw(storage_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(device_dto(&stored, &namespace)?))
}

/// `DELETE /devices/:id` — deregister a device (Admin + `device-manage`).
pub async fn delete_device_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    // Confirm visibility/existence in-namespace before deleting, for a clean 404.
    load_device(&auth, &namespace, &id).await?;

    let command = Command::new(
        auth.principal.clone(),
        DEVICE_MANAGE,
        Id::from_raw(device_storage_id(&namespace, &id)),
        Change::Delete,
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_device_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Read a device on the scoped session, `404` if absent or out of scope.
async fn load_device(auth: &Authenticated, namespace: &str, id: &str) -> Result<Record, ApiError> {
    let storage_id = device_storage_id(namespace, id);
    let record = read_record_on_session(&auth.session, &Id::from_raw(storage_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    // A record at this id that is not a device must not masquerade as one.
    if record.content.get("kind").and_then(Value::as_str) != Some(DEVICE_KIND) {
        return Err(ApiError::NotFound);
    }
    Ok(record)
}

/// The namespace-prefixed storage id for a device (`{namespace}_{id}`).
fn device_storage_id(namespace: &str, id: &str) -> String {
    format!("{namespace}_{id}")
}

/// Build the device record content: the `device` discriminator plus the fields.
///
/// The device *class* is stored under `class` (not `kind`, which is reserved as
/// the collection discriminator); the DTO surfaces `class` as its `kind` field.
fn device_content(label: &str, class: &str, metadata: &BTreeMap<String, Value>) -> Value {
    let mut map = Map::new();
    map.insert("kind".to_owned(), Value::String(DEVICE_KIND.to_owned()));
    map.insert("label".to_owned(), Value::String(label.to_owned()));
    map.insert("class".to_owned(), Value::String(class.to_owned()));
    map.insert(
        "metadata".to_owned(),
        Value::Object(metadata.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    );
    Value::Object(map)
}

/// Project a device record into its DTO, stripping the namespace prefix from id.
fn device_dto(record: &Record, namespace: &str) -> Result<DeviceDto, ApiError> {
    let content = &record.content;
    let label = content
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let class = content
        .get("class")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let metadata = content
        .get("metadata")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    let full_id = record.id.to_string();
    let prefix = format!("{namespace}_");
    let id = full_id.strip_prefix(&prefix).unwrap_or(&full_id).to_owned();
    Ok(DeviceDto {
        id,
        namespace: record.namespace.clone(),
        label,
        kind: class,
        metadata,
    })
}

/// Map a gate failure to its transport status: a denied grant is `403`, a failed
/// contract `422`, anything else internal.
fn map_device_error(error: rubix_gate::GateError) -> ApiError {
    match error {
        rubix_gate::GateError::CommandDenied(reason) => ApiError::Forbidden(reason),
        rubix_gate::GateError::Validation(reason) => ApiError::Unprocessable(reason),
        other => ApiError::Internal(other.to_string()),
    }
}
