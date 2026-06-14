//! Dashboard variables: the model that parameterises one dashboard across a
//! fleet (docs/design/variables-and-templating.md §1).
//!
//! A [`Variable`] lives in the dashboard's stored config (it travels with
//! export/import, so it is part of the dashboard snapshot rather than a separate
//! relational table). A variable bar lets a user pick values; widgets re-query
//! against the selection. The variable's *value* reaches SQL only as a bound
//! parameter via the `rubix-query` interpolation engine — never spliced into the
//! SQL text. This module is the pure model; resolution and the UI live on the
//! frontend, and interpolation lives in `rubix-query`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The kind of a dashboard variable: a closed enum, mirroring the [`crate::WidgetKind`]
/// pattern — adding a kind is a deliberate DTO + UI change. Built-in,
/// context-sourced variables (`$__org`/`$__site`/`$__user`, and `$__from`/`$__to`
/// from the time-range scope) are read-only and resolved by the frontend; they
/// are not authored as stored variables, so they are not members of this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VariableKind {
    /// A fixed value, usually hidden. Resolves to its configured constant.
    Constant,
    /// A static, operator-authored option list.
    Custom,
    /// Options come from running SQL (one column becomes the option list). The
    /// SQL may reference another variable, which is how cascading works.
    Query,
    /// Options are the org's datasources of a given kind, so the variable can
    /// drive which datasource widgets target.
    Datasource,
    /// Options are the org's sites; the headline rubix kind, resolves to
    /// `site_id` — the natural fleet axis.
    Site,
    /// A list of durations driving `$__interval` overrides (pairs with the
    /// time-range scope).
    Interval,
    /// Free text entry.
    Textbox,
    /// A value sourced from the page context (the nav node, bare URL params, the
    /// board's tags, or a nav node's `context.values`) rather than authored
    /// options (docs/design/page-context-and-nav.md §2). Lets one board, mounted
    /// at two nav nodes, resolve different values without a second board. The
    /// resolved value still reaches SQL only as a bound parameter.
    Context,
}

/// Per-kind variable configuration. Tagged on `kind` so the wire shape is
/// self-describing and a kind only carries the fields it needs. The server
/// treats this as opaque transport — it persists and returns it but does not
/// resolve options (resolution is the frontend's job, against live data); only
/// the interpolation engine ever reads a *resolved* value, and that arrives as a
/// separate `QueryVariable` on the query request, not from this config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VariableConfig {
    /// A fixed value.
    Constant {
        /// The constant the variable always resolves to.
        value: Value,
    },
    /// A static option list.
    Custom {
        /// The authored options, in display order.
        options: Vec<String>,
    },
    /// Options from SQL run against a datasource (`datasource_id`) or the
    /// canonical tables when `datasource_id` is absent. The SQL may reference
    /// other variables (`WHERE site_id = '$site'`), which establishes the
    /// dependency the frontend resolves topologically.
    Query {
        /// The option query; its first column becomes the option list.
        sql: String,
        /// The datasource this query runs against; absent → canonical `/query`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        datasource_id: Option<String>,
    },
    /// Datasources of a given kind become the options.
    Datasource {
        /// The datasource kind to list (e.g. `"timescaledb"`); absent → all.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        datasource_kind: Option<String>,
    },
    /// Sites under the org become the options; resolves to `site_id`.
    Site {},
    /// A list of authored duration tokens (e.g. `"1m"`, `"5m"`, `"1h"`).
    Interval {
        /// The duration options, in display order.
        options: Vec<String>,
    },
    /// Free text; no options.
    Textbox {},
    /// A page-context value (docs/design/page-context-and-nav.md §2). `source`
    /// selects which context layer to read; `key` addresses one value within it.
    /// The server treats this as opaque transport like every other config — the
    /// frontend assembles the `PageContext` and resolves the value, which then
    /// arrives as a bound `QueryVariable` on the query request.
    Context {
        /// The context layer to read from.
        source: ContextSource,
        /// The key within the source (a variable/tag name, a bare URL param, or
        /// `slug`/`name`/`path[n]` for the `nav` source).
        key: String,
    },
}

/// Which page-context layer a `context` variable reads from
/// (docs/design/page-context-and-nav.md §2). A closed enum so the resolvable
/// sources stay explicit and testable; precedence between sources is the
/// frontend resolution layer's concern, not the wire shape's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// The nav node the page opened under (`key` = `slug`|`name`|`path[n]`).
    Nav,
    /// A bare URL query param (`?building=…`); not the `var-*` namespace.
    Url,
    /// The board's own tag value for `key`.
    Tag,
    /// The nav node's `context.values[key]` override.
    Values,
}

impl VariableConfig {
    /// The [`VariableKind`] this config carries, so a `Variable`'s declared kind
    /// and its config can be checked for agreement at the validation boundary.
    pub fn kind(&self) -> VariableKind {
        match self {
            VariableConfig::Constant { .. } => VariableKind::Constant,
            VariableConfig::Custom { .. } => VariableKind::Custom,
            VariableConfig::Query { .. } => VariableKind::Query,
            VariableConfig::Datasource { .. } => VariableKind::Datasource,
            VariableConfig::Site {} => VariableKind::Site,
            VariableConfig::Interval { .. } => VariableKind::Interval,
            VariableConfig::Textbox {} => VariableKind::Textbox,
            VariableConfig::Context { .. } => VariableKind::Context,
        }
    }
}

/// One dashboard variable. `name` is referenced in widget SQL as `$name` /
/// `${name}`; `current` holds the selected value(s). The variable is part of the
/// dashboard snapshot (stored in the dashboard's JSON config), so it round-trips
/// through export/import unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Variable {
    /// The variable name as referenced in SQL, without the leading `$`.
    pub name: String,
    /// Human-facing label; falls back to `name` in the UI when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The variable kind. Must agree with `config`'s tag (see
    /// [`Variable::validate`]).
    pub kind: VariableKind,
    /// Per-kind configuration (tagged on its own `kind`).
    pub config: VariableConfig,
    /// The currently-selected value(s). A scalar for single-select, an array for
    /// multi-select; the frontend resolution layer maintains it.
    #[serde(default)]
    pub current: Value,
    /// Whether the variable allows selecting multiple values.
    #[serde(default)]
    pub multi: bool,
    /// Whether to offer an "All" option that expands to every option.
    #[serde(default)]
    pub include_all: bool,
    /// Whether the variable is hidden from the variable bar (still resolvable;
    /// e.g. a `constant` used only inside other variables' SQL).
    #[serde(default)]
    pub hidden: bool,
}

impl Variable {
    /// Validate a variable for persistence: a non-empty name and a `config`
    /// whose tag matches the declared `kind`. The interpolation engine already
    /// owns value safety, so this guards only the structural contract.
    pub fn validate(&self) -> Result<(), VariableError> {
        if self.name.trim().is_empty() {
            return Err(VariableError::EmptyName);
        }
        if self.config.kind() != self.kind {
            return Err(VariableError::KindMismatch {
                name: self.name.clone(),
                declared: self.kind,
                config: self.config.kind(),
            });
        }
        Ok(())
    }
}

/// Validate every variable on a dashboard, also rejecting duplicate names (a
/// duplicate would make `$name` ambiguous at interpolation time).
pub fn validate_variables(variables: &[Variable]) -> Result<(), VariableError> {
    let mut seen: Vec<&str> = Vec::with_capacity(variables.len());
    for variable in variables {
        variable.validate()?;
        if seen.contains(&variable.name.as_str()) {
            return Err(VariableError::DuplicateName {
                name: variable.name.clone(),
            });
        }
        seen.push(&variable.name);
    }
    Ok(())
}

/// A dashboard-variable validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VariableError {
    /// A variable name was empty or whitespace-only.
    #[error("variable name must not be empty")]
    EmptyName,
    /// The declared `kind` disagreed with the `config` tag.
    #[error("variable `{name}` declares kind {declared:?} but config is {config:?}")]
    KindMismatch {
        name: String,
        declared: VariableKind,
        config: VariableKind,
    },
    /// Two variables share a name.
    #[error("duplicate variable name `{name}`")]
    DuplicateName { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn site_var(name: &str) -> Variable {
        Variable {
            name: name.to_string(),
            label: None,
            kind: VariableKind::Site,
            config: VariableConfig::Site {},
            current: Value::Null,
            multi: false,
            include_all: false,
            hidden: false,
        }
    }

    #[test]
    fn config_kind_round_trips_for_every_variant() {
        assert_eq!(
            VariableConfig::Constant { value: json!(1) }.kind(),
            VariableKind::Constant
        );
        assert_eq!(
            VariableConfig::Custom { options: vec![] }.kind(),
            VariableKind::Custom
        );
        assert_eq!(
            VariableConfig::Query {
                sql: "SELECT 1".into(),
                datasource_id: None
            }
            .kind(),
            VariableKind::Query
        );
        assert_eq!(
            VariableConfig::Datasource {
                datasource_kind: None
            }
            .kind(),
            VariableKind::Datasource
        );
        assert_eq!(VariableConfig::Site {}.kind(), VariableKind::Site);
        assert_eq!(
            VariableConfig::Interval { options: vec![] }.kind(),
            VariableKind::Interval
        );
        assert_eq!(VariableConfig::Textbox {}.kind(), VariableKind::Textbox);
        assert_eq!(
            VariableConfig::Context {
                source: ContextSource::Values,
                key: "site".into()
            }
            .kind(),
            VariableKind::Context
        );
    }

    #[test]
    fn context_config_round_trips_on_wire() {
        let cfg = VariableConfig::Context {
            source: ContextSource::Nav,
            key: "slug".into(),
        };
        let wire = serde_json::to_value(&cfg).unwrap();
        assert_eq!(
            wire,
            json!({ "kind": "context", "source": "nav", "key": "slug" })
        );
        let back: VariableConfig = serde_json::from_value(wire).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn context_variable_validates_like_any_other() {
        let v = Variable {
            name: "site".into(),
            label: None,
            kind: VariableKind::Context,
            config: VariableConfig::Context {
                source: ContextSource::Values,
                key: "site".into(),
            },
            current: Value::Null,
            multi: false,
            include_all: false,
            hidden: false,
        };
        assert!(v.validate().is_ok());
        // Declared kind must still match the config tag.
        let bad = Variable {
            kind: VariableKind::Site,
            ..v
        };
        assert!(matches!(
            bad.validate(),
            Err(VariableError::KindMismatch { .. })
        ));
    }

    #[test]
    fn valid_variable_passes() {
        assert!(site_var("site").validate().is_ok());
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut v = site_var("  ");
        v.name = "  ".into();
        assert_eq!(v.validate(), Err(VariableError::EmptyName));
    }

    #[test]
    fn kind_config_mismatch_is_rejected() {
        let v = Variable {
            kind: VariableKind::Query,
            ..site_var("site")
        };
        assert!(matches!(
            v.validate(),
            Err(VariableError::KindMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let vars = vec![site_var("site"), site_var("site")];
        assert_eq!(
            validate_variables(&vars),
            Err(VariableError::DuplicateName {
                name: "site".into()
            })
        );
    }

    #[test]
    fn config_is_tagged_on_kind_for_wire() {
        let v = site_var("site");
        let wire = serde_json::to_value(&v.config).unwrap();
        assert_eq!(wire, json!({ "kind": "site" }));
    }

    #[test]
    fn query_config_carries_sql_and_optional_datasource() {
        let cfg = VariableConfig::Query {
            sql: "SELECT slug FROM sites WHERE site_id = '$parent'".into(),
            datasource_id: Some("ds-1".into()),
        };
        let wire = serde_json::to_value(&cfg).unwrap();
        assert_eq!(
            wire,
            json!({
                "kind": "query",
                "sql": "SELECT slug FROM sites WHERE site_id = '$parent'",
                "datasource_id": "ds-1"
            })
        );
    }
}
