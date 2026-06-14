//! The stored-rule model.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A declared parameter and whether it is required.
///
/// The schema is intentionally lightweight in v1: a name and a required flag per
/// parameter, plus an optional human description. It exists so a composition
/// mismatch (a caller omitting a required param) fails clearly at call time
/// rather than opaquely inside the script. The integrating session can widen
/// this to typed/constrained params without changing the resolver contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSchema {
    /// Declared parameters keyed by name; the value is `required`.
    #[serde(default)]
    pub params: BTreeMap<String, ParamSpec>,
}

/// One declared parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSpec {
    /// Whether the caller must supply this parameter.
    #[serde(default)]
    pub required: bool,
    /// Optional human description.
    #[serde(default)]
    pub description: Option<String>,
}

impl ParamSchema {
    /// An empty schema (no declared parameters).
    pub fn empty() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    /// The names of parameters declared `required`.
    pub fn required_names(&self) -> impl Iterator<Item = &str> {
        self.params
            .iter()
            .filter(|(_, s)| s.required)
            .map(|(n, _)| n.as_str())
    }
}

/// A saved rule: a named, parameterized Rhai script returning a verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredRule {
    /// Stable identifier (the id a board node references).
    pub id: String,
    /// Unique name used for composition (`rule("temp-high", …)`).
    pub name: String,
    /// The Rhai source.
    pub script: String,
    /// Declared parameter schema.
    #[serde(default = "ParamSchema::empty")]
    pub params: ParamSchema,
}

impl StoredRule {
    /// Build a stored rule with an empty parameter schema.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        script: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            script: script.into(),
            params: ParamSchema::empty(),
        }
    }
}
