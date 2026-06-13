//! Declarative schema for the built-in board components: their ports and the
//! shape of each component's `config` blob. This is the single source of truth
//! the server exposes (over `GET /api/v1/boards/components`) so the editor can
//! render a config form for a node without hardcoding field names — adding a
//! component here surfaces it to the UI.
//!
//! The declarations must stay faithful to what each actor in [`crate::node`]
//! actually reads from `config`; the `component_schema_matches_actor` tests
//! pin the port lists against the live `ActorBase` declarations.

use serde::{Deserialize, Serialize};

/// The kind of a config field's value, driving which form control the editor
/// renders and how the value is validated before a board is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// Free text (e.g. a prompt or finding message).
    String,
    /// A point keyexpr — text, but the editor may offer point completion.
    Keyexpr,
    /// An integer, optionally bounded by `min`/`max`.
    Integer,
    /// A floating-point number.
    Number,
    /// A boolean toggle.
    Boolean,
    /// One of a fixed set of string tokens (see `options`).
    Enum,
    /// A free-form JSON object (e.g. a rule's `params` map).
    Json,
}

/// One configurable field on a component's `config` map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    /// The key in the node's `config` map.
    pub name: String,
    /// Human label for the editor.
    pub label: String,
    /// What kind of value this field holds.
    pub field_type: FieldType,
    /// Whether the board fails closed without this field set.
    pub required: bool,
    /// Default applied by the actor when the field is absent, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Allowed tokens for an [`FieldType::Enum`] field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Inclusive lower bound for [`FieldType::Integer`]/[`FieldType::Number`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Inclusive upper bound for [`FieldType::Integer`]/[`FieldType::Number`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// One-line help shown under the control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl ConfigField {
    fn new(name: &str, label: &str, field_type: FieldType, required: bool) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            field_type,
            required,
            default: None,
            options: Vec::new(),
            min: None,
            max: None,
            help: None,
        }
    }

    fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    fn with_options(mut self, options: &[&str]) -> Self {
        self.options = options.iter().map(|s| s.to_string()).collect();
        self
    }

    fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    fn with_help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }
}

/// The semantic value a port carries, used to validate connections in the
/// editor. These mirror the `message_to_value` scalar/non-scalar boundary in
/// [`crate::node`] rather than reflow's full message taxonomy — the actors only
/// distinguish these classes, so the editor should too. A connection is allowed
/// when the source's type is compatible with the target's (see `accepts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortType {
    /// A control tick with no data payload (a trigger/clock edge).
    Flow,
    /// A scalar point value: boolean, number, or string.
    Scalar,
    /// A structured JSON payload (e.g. a history array).
    Object,
    /// An error string emitted on a node's `error` outport.
    Error,
}

impl PortType {
    /// Whether a value of `self` may feed a `target` inport. A `flow` tick can
    /// drive any inport (it just fires the node); otherwise the classes must
    /// match. `error` only connects to `error`-typed inports (none today, so
    /// error outports are terminal — surfaced, not wired onward by accident).
    pub fn accepts(self, target: PortType) -> bool {
        match self {
            PortType::Flow => true,
            other => other == target,
        }
    }
}

/// One node port: an `id` matching the actor's declared in/out port name, a
/// display `label`, and the `port_type` the editor validates connections with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSchema {
    pub id: String,
    pub label: String,
    pub port_type: PortType,
}

impl PortSchema {
    fn new(id: &str, label: &str, port_type: PortType) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            port_type,
        }
    }
}

/// How a component presents and what it does, for editor grouping and labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    /// A source: reads/produces data (`read_point`, `query_his`, `trigger`).
    Source,
    /// Pure logic over inputs.
    Logic,
    /// A sink: writes/acts on the world (`write_point`, `emit_spark`).
    Sink,
    /// Calls the embedded agent.
    Agent,
}

/// The full schema for one board component: identity, ports, and config shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// The component name used in [`super::BoardNode::component`].
    pub component: String,
    /// Display name for the palette.
    pub label: String,
    /// One-line description.
    pub description: String,
    /// Editor grouping / styling class.
    pub kind: ComponentKind,
    pub inports: Vec<PortSchema>,
    pub outports: Vec<PortSchema>,
    pub config: Vec<ConfigField>,
}

/// The schema catalogue for every built-in component, in palette order. This is
/// the value served by the components endpoint; it is the editor's only source
/// of port and config truth.
pub fn component_schemas() -> Vec<ComponentSchema> {
    use ComponentKind::*;
    use FieldType::*;

    vec![
        ComponentSchema {
            component: "read_point".into(),
            label: "Point Read".into(),
            description: "Read a point's current value by keyexpr and emit it.".into(),
            kind: Source,
            inports: vec![PortSchema::new("trigger", "trigger", PortType::Flow)],
            outports: vec![
                PortSchema::new("output", "output", PortType::Scalar),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![ConfigField::new("point", "Point", Keyexpr, true)
                .with_help("Keyexpr of the point to read.")],
        },
        ComponentSchema {
            component: "write_point".into(),
            label: "Point Write".into(),
            description: "Command a point's priority slot with the input value.".into(),
            kind: Sink,
            inports: vec![PortSchema::new("value", "value", PortType::Scalar)],
            outports: vec![
                PortSchema::new("output", "output", PortType::Scalar),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("point", "Point", Keyexpr, true)
                    .with_help("Keyexpr of the point to command."),
                ConfigField::new("priority", "Priority", Integer, false)
                    .with_default(serde_json::json!(16))
                    .with_range(1.0, 16.0)
                    .with_help("Priority array slot (1 highest, 16 lowest)."),
            ],
        },
        ComponentSchema {
            component: "query_his".into(),
            label: "History Query".into(),
            description: "Fetch recent history for a point as a JSON array.".into(),
            kind: Source,
            inports: vec![PortSchema::new("trigger", "trigger", PortType::Flow)],
            outports: vec![
                PortSchema::new("output", "output", PortType::Object),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("point", "Point", Keyexpr, true)
                    .with_help("Keyexpr of the point to query."),
                ConfigField::new("limit", "Limit", Integer, false)
                    .with_default(serde_json::json!(100))
                    .with_range(1.0, 10_000.0)
                    .with_help("Maximum samples to return."),
            ],
        },
        ComponentSchema {
            component: "datasource".into(),
            label: "Datasource Query".into(),
            description: "Run a read-only query against an external SQL datasource \
                          (TimescaleDB/Postgres) as a JSON grid."
                .into(),
            kind: Source,
            inports: vec![PortSchema::new("trigger", "trigger", PortType::Flow)],
            outports: vec![
                PortSchema::new("output", "output", PortType::Object),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("datasource", "Datasource", String, true)
                    .with_help("Registered datasource id to read from."),
                ConfigField::new("sql", "SQL", String, false)
                    .with_help("Native SQL with $1-style params; set this or `named`, not both."),
                ConfigField::new("named", "Named query", String, false).with_help(
                    "Operator-registered named query to invoke; set this or `sql`, not both.",
                ),
                ConfigField::new("params", "Params", Json, false)
                    .with_help("JSON array of typed bound parameters ([{type,value}, …])."),
            ],
        },
        ComponentSchema {
            component: "rule".into(),
            label: "Rule".into(),
            description: "Evaluate a sandboxed rule over query rows; flag a finding.".into(),
            kind: Logic,
            inports: vec![PortSchema::new("input", "input", PortType::Object)],
            outports: vec![
                PortSchema::new("finding", "finding", PortType::Object),
                PortSchema::new("clear", "clear", PortType::Flow),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("script", "Script", String, false)
                    .with_help("Inline Rhai rule; set this or `rule`, not both."),
                ConfigField::new("rule", "Stored rule", String, false)
                    .with_help("Stored rule id to run; set this or `script`, not both."),
                ConfigField::new("params", "Params", Json, false)
                    .with_help("JSON object exposed to the script as `params`."),
                ConfigField::new("max_rows", "Max rows", Integer, false)
                    .with_default(serde_json::json!(10_000))
                    .with_range(1.0, 1_000_000.0)
                    .with_help("Input row cap; a larger input fails as a truncation breach."),
            ],
        },
        ComponentSchema {
            component: "trigger".into(),
            label: "Trigger".into(),
            description: "Self-paced timing source; fires on its configured cadence.".into(),
            kind: Source,
            inports: vec![PortSchema::new("trigger", "trigger", PortType::Flow)],
            outports: vec![
                PortSchema::new("boot", "boot", PortType::Scalar),
                PortSchema::new("count", "count", PortType::Scalar),
                PortSchema::new("output", "output", PortType::Scalar),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("every", "Every", Number, false)
                    .with_default(serde_json::json!(1))
                    .with_range(0.0, f64::MAX)
                    .with_help("Period length (must be positive)."),
                ConfigField::new("unit", "Unit", Enum, false)
                    .with_default(serde_json::json!("sec"))
                    .with_options(&["sec", "min", "hours"])
                    .with_help("Period unit."),
            ],
        },
        ComponentSchema {
            component: "agent_call".into(),
            label: "Agent Call".into(),
            description: "Ask the embedded agent to act; detached or awaited.".into(),
            kind: Agent,
            inports: vec![PortSchema::new("value", "value", PortType::Scalar)],
            outports: vec![
                PortSchema::new("output", "output", PortType::Scalar),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("prompt", "Prompt", String, false)
                    .with_help("Static prompt; overridden by a connected `value` input."),
                ConfigField::new("thread", "Thread", String, false)
                    .with_default(serde_json::json!("board-agent-call"))
                    .with_help("Thread that groups repeated calls."),
                ConfigField::new("await", "Await response", Boolean, false)
                    .with_default(serde_json::json!(false))
                    .with_help("Block on the agent and emit its response, vs fire-and-forget."),
            ],
        },
        ComponentSchema {
            component: "emit_spark".into(),
            label: "Emit Finding".into(),
            description: "Record a rule-board finding (spark) for a site.".into(),
            kind: Sink,
            inports: vec![
                PortSchema::new("value", "value", PortType::Scalar),
                PortSchema::new("finding", "finding", PortType::Object),
            ],
            outports: vec![
                PortSchema::new("output", "output", PortType::Scalar),
                PortSchema::new("error", "error", PortType::Error),
            ],
            config: vec![
                ConfigField::new("site", "Site", Keyexpr, true)
                    .with_help("`{org}/{site}` keyexpr prefix the finding belongs to."),
                ConfigField::new("rule", "Rule", String, true)
                    .with_help("Rule identifier for the finding."),
                ConfigField::new("severity", "Severity", Enum, false)
                    .with_default(serde_json::json!("warning"))
                    .with_options(&["info", "warning", "fault"])
                    .with_help("Default severity; a connected `finding` input overrides it."),
                ConfigField::new("message", "Message", String, false)
                    .with_help("Static finding text; a `value` or `finding` input overrides it."),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::COMPONENTS;
    use std::collections::HashSet;

    /// The schema catalogue must cover exactly the registered components — no
    /// more (a schema for a component the engine can't build), no fewer (a
    /// component the editor can place but can't configure). Adding a component
    /// to the registry without a schema here fails this test.
    #[test]
    fn schemas_cover_exactly_the_registry() {
        let schemas = component_schemas();
        let schema_names: HashSet<&str> =
            schemas.iter().map(|s| s.component.as_str()).collect();
        let registry_names: HashSet<&str> = COMPONENTS.iter().copied().collect();
        assert_eq!(
            schema_names, registry_names,
            "component schema catalogue diverged from the actor registry"
        );
    }

    /// Every component carries an `error` outport (the actors all emit on it),
    /// and no port id is duplicated within a component.
    #[test]
    fn every_component_has_error_out_and_unique_ports() {
        for s in component_schemas() {
            assert!(
                s.outports.iter().any(|p| p.id == "error"),
                "{} missing `error` outport",
                s.component
            );
            let mut ports = HashSet::new();
            for p in s.inports.iter().chain(s.outports.iter()) {
                assert!(ports.insert(&p.id), "{} duplicate port {}", s.component, p.id);
            }
        }
    }

    /// Connection rules: a flow tick drives anything; data classes must match;
    /// error outports are terminal (no inport accepts `error`).
    #[test]
    fn port_type_accepts_is_grounded() {
        assert!(PortType::Flow.accepts(PortType::Scalar));
        assert!(PortType::Flow.accepts(PortType::Object));
        assert!(PortType::Scalar.accepts(PortType::Scalar));
        assert!(!PortType::Scalar.accepts(PortType::Object));
        assert!(!PortType::Object.accepts(PortType::Scalar));
        // No component declares an `error` inport, so error stays terminal.
        let has_error_inport = component_schemas()
            .iter()
            .flat_map(|s| s.inports.iter())
            .any(|p| p.port_type == PortType::Error);
        assert!(!has_error_inport, "error outports must remain terminal");
    }

    /// Enum fields must list options; defaults that exist must be one of them.
    #[test]
    fn enum_fields_are_well_formed() {
        for s in component_schemas() {
            for f in &s.config {
                if f.field_type == FieldType::Enum {
                    assert!(!f.options.is_empty(), "{}.{} enum has no options", s.component, f.name);
                    if let Some(serde_json::Value::String(d)) = &f.default {
                        assert!(
                            f.options.contains(d),
                            "{}.{} default {d:?} not in options",
                            s.component,
                            f.name
                        );
                    }
                }
            }
        }
    }
}
