//! Wire types for the stored-rule routes. `RuleRecord` is the domain type; these
//! shape its create/update request and its JSON response. A rule is owned by an
//! `org` (path param) and optionally a `site` (`?site_id=`) — the uniform scope
//! dashboards/flows share; a site rule overrides the org-level one of the same
//! name during a board run.

use rubix_rules::ParamSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::store::RuleRecord;

/// Create a rule under an org, optionally pinned to a site. `name` is unique per
/// scope `(org, site_id)`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRule {
    /// The single site this rule is for; omit for an org-level rule.
    #[serde(default)]
    pub site_id: Option<Uuid>,
    /// Composition name (`rule("temp-high", …)`), unique per scope.
    pub name: String,
    /// The Rhai rule script (returns a verdict).
    pub script: String,
    /// Declared parameter schema; defaults to empty.
    #[serde(default = "ParamSchema::empty")]
    #[schema(value_type = Object)]
    pub params: ParamSchema,
}

/// Scope query for the slug/name-addressed rule routes (`?site_id=`): omit for
/// the org-level rule, set to address a site-scoped one.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RuleScope {
    #[serde(default)]
    pub site_id: Option<Uuid>,
}

/// Replace a rule's script and/or params. `org`/`site_id`/`name` identify the
/// rule and are not edited here (renaming/re-scoping is a delete + create).
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_id: Option<Uuid>,
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
            site_id: r.site_id,
            name: r.name,
            script: r.script,
            params: r.params,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
