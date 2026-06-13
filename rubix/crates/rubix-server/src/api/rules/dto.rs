//! Wire types for the org-scoped stored-rule routes. `RuleRecord` is the domain
//! type; these shape its create/update request and its JSON response.

use rubix_rules::ParamSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::store::RuleRecord;

/// Create a rule under an org. `name` is unique within the org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRule {
    /// Composition name (`rule("temp-high", …)`), unique per org.
    pub name: String,
    /// The Rhai rule script (returns a verdict).
    pub script: String,
    /// Declared parameter schema; defaults to empty.
    #[serde(default = "ParamSchema::empty")]
    #[schema(value_type = Object)]
    pub params: ParamSchema,
}

/// Replace a rule's script and/or params. `name` and `org` identify the rule and
/// are not edited here (renaming is a delete + create).
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRule {
    pub script: String,
    #[serde(default = "ParamSchema::empty")]
    #[schema(value_type = Object)]
    pub params: ParamSchema,
}

/// A stored rule as returned to the caller.
#[derive(Debug, Serialize, ToSchema)]
pub struct RuleView {
    pub id: Uuid,
    pub org: String,
    pub name: String,
    pub script: String,
    #[schema(value_type = Object)]
    pub params: ParamSchema,
    pub created_at: String,
}

impl From<RuleRecord> for RuleView {
    fn from(r: RuleRecord) -> Self {
        RuleView {
            id: r.id,
            org: r.org,
            name: r.name,
            script: r.script,
            params: r.params,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
