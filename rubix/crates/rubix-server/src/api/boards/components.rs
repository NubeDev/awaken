//! GET /api/v1/boards/components — the board component catalogue: every
//! built-in node's ports and config-field schema. The flow editor drives its
//! palette and per-node config form from this, so adding a component (with a
//! schema in `rubix-flow`) surfaces it to the UI without a client change.
//!
//! The catalogue is static (it describes compiled-in actors), so this handler
//! takes no store and does no IO.

use axum::Json;
use rubix_flow::{
    component_schemas, ComponentKind, ComponentSchema, ConfigField, FieldType, PortSchema, PortType,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::ApiError;

/// A component's port: id matches the node's wire port name.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PortView {
    pub id: String,
    pub label: String,
    /// `flow` | `scalar` | `object` | `error` — the editor validates a
    /// connection by checking the source's type accepts the target's.
    pub port_type: String,
}

impl From<PortSchema> for PortView {
    fn from(p: PortSchema) -> Self {
        PortView {
            id: p.id,
            label: p.label,
            port_type: port_type_token(p.port_type).to_string(),
        }
    }
}

fn port_type_token(t: PortType) -> &'static str {
    match t {
        PortType::Flow => "flow",
        PortType::Scalar => "scalar",
        PortType::Object => "object",
        PortType::Error => "error",
    }
}

/// One configurable field on a component's `config` map.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ConfigFieldView {
    pub name: String,
    pub label: String,
    /// `string` | `keyexpr` | `integer` | `number` | `boolean` | `enum`.
    pub field_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl From<ConfigField> for ConfigFieldView {
    fn from(f: ConfigField) -> Self {
        ConfigFieldView {
            name: f.name,
            label: f.label,
            field_type: field_type_token(f.field_type).to_string(),
            required: f.required,
            default: f.default,
            options: f.options,
            min: f.min,
            max: f.max,
            help: f.help,
        }
    }
}

fn field_type_token(t: FieldType) -> &'static str {
    match t {
        FieldType::String => "string",
        FieldType::Keyexpr => "keyexpr",
        FieldType::Integer => "integer",
        FieldType::Number => "number",
        FieldType::Boolean => "boolean",
        FieldType::Enum => "enum",
        FieldType::Json => "json",
    }
}

fn kind_token(k: ComponentKind) -> &'static str {
    match k {
        ComponentKind::Source => "source",
        ComponentKind::Logic => "logic",
        ComponentKind::Sink => "sink",
        ComponentKind::Agent => "agent",
    }
}

/// A board component as the editor sees it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ComponentView {
    pub component: String,
    pub label: String,
    pub description: String,
    /// `source` | `logic` | `sink` | `agent`.
    pub kind: String,
    pub inports: Vec<PortView>,
    pub outports: Vec<PortView>,
    pub config: Vec<ConfigFieldView>,
}

impl From<ComponentSchema> for ComponentView {
    fn from(s: ComponentSchema) -> Self {
        ComponentView {
            component: s.component,
            label: s.label,
            description: s.description,
            kind: kind_token(s.kind).to_string(),
            inports: s.inports.into_iter().map(PortView::from).collect(),
            outports: s.outports.into_iter().map(PortView::from).collect(),
            config: s.config.into_iter().map(ConfigFieldView::from).collect(),
        }
    }
}

#[utoipa::path(get, path = "/api/v1/boards/components", tag = "boards",
    responses((status = 200, body = [ComponentView])))]
pub(crate) async fn list_components() -> Result<Json<Vec<ComponentView>>, ApiError> {
    let views: Vec<ComponentView> = component_schemas()
        .into_iter()
        .map(ComponentView::from)
        .collect();
    Ok(Json(views))
}
