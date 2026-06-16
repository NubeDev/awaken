//! Wire shapes for the rule resource.
//!
//! A rule is the deterministic decision unit (`rubix/docs/SCOPE.md`, "Rhai —
//! rules and insights"): a Rhai script, the input bindings that resolve its window
//! values from `rubix-query`, the sub-rules it composes, and the insight kind it
//! emits. Rules persist as `kind:"rule"` records over the generic record surface
//! (the same gate/audit/scoped-session path charts and saved queries ride), so no
//! new table is introduced — the transport just owns the rule-shaped content and a
//! dedicated [`RuleDefine`](rubix_gate::Capability::RuleDefine) gate.
//!
//! The DTOs here are the bridge between that stored content and the `rubix-rules`
//! crate types: a [`BindingDto`] maps to a [`rubix_rules::Binding`], and a rule's
//! content reconstructs a [`rubix_rules::Rule`] for a dry-run.

use rubix_core::{Id, Record};
use rubix_query::{CanonicalTable, Grain};
use rubix_rules::{Aggregate, Binding, Rule};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The `kind` every rule record carries in its content (the list filter, §1a).
pub const RULE_KIND: &str = "rule";

/// One declared input a rule's script reads as a time-window value.
///
/// Mirrors [`rubix_rules::Binding`] on the wire: the script variable `name`, the
/// canonical `table` and numeric `field` rolled up, the bucket `grain`, and the
/// `aggregate` of the latest bucket the rule decides on. The string enums map
/// one-to-one to the crate's `CanonicalTable`/`Grain`/`Aggregate`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BindingDto {
    /// The script variable name this value binds to.
    pub name: String,
    /// The canonical table the series is read from
    /// (`records`/`tags`/`audit`/`insights`/`trace_summary`).
    pub table: String,
    /// The numeric `content.<field>` series rolled up.
    pub field: String,
    /// The bucket width (`minute`/`hour`/`day`/`week`).
    pub grain: String,
    /// The bucket aggregate (`avg`/`min`/`max`/`sum`/`count`/`first`/`last`).
    pub aggregate: String,
    /// An optional `content` key to narrow the series on (e.g. `"measure"`).
    ///
    /// When set together with [`filter_value`](BindingDto::filter_value), the
    /// rollup is restricted to rows whose `content.<filter_field>` equals it — so
    /// a binding on the shared `value` field can target one `measure`. Both must
    /// be present for the filter to apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_field: Option<String>,
    /// The exact value `content.<filter_field>` must equal for the filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_value: Option<String>,
}

impl BindingDto {
    /// Map this wire binding to a [`rubix_rules::Binding`], validating its enums.
    ///
    /// # Errors
    /// Returns the offending value as `Err` when `table`, `grain`, or `aggregate`
    /// is not one of the known variants — the caller maps it to a `400`.
    pub fn to_binding(&self) -> Result<Binding, String> {
        let binding = Binding::new(
            self.name.clone(),
            parse_table(&self.table)?,
            self.field.clone(),
            parse_grain(&self.grain)?,
            parse_aggregate(&self.aggregate)?,
        );
        // A filter applies only when both halves are present; a lone field or
        // value is treated as no filter rather than an error.
        match (&self.filter_field, &self.filter_value) {
            (Some(key), Some(value)) if !key.is_empty() => {
                Ok(binding.filtered_by(key.clone(), value.clone()))
            }
            _ => Ok(binding),
        }
    }
}

/// Build a [`rubix_rules::Rule`] from its parts, validating the binding enums.
///
/// Shared by the dry-run path (which builds the draft rule and each stored
/// sub-rule it composes into one registry): `name` is the rule's id and
/// composition handle, `subrules` the names it may `invoke`, `output` the insight
/// kind. The script is not compiled here — the dry-run engine surfaces a compile
/// error when it runs.
///
/// # Errors
/// Returns the offending binding value when an input names an unknown
/// table/grain/aggregate.
pub fn build_rule(
    name: &str,
    script: &str,
    inputs: &[BindingDto],
    subrules: &[String],
    output: &str,
) -> Result<Rule, String> {
    let bindings = inputs
        .iter()
        .map(BindingDto::to_binding)
        .collect::<Result<Vec<_>, _>>()?;
    let subrule_ids = subrules
        .iter()
        .map(|s| Id::from_raw(s.as_str()))
        .collect::<Vec<_>>();
    Ok(Rule::new(Id::from_raw(name), script, bindings, output).composing(subrule_ids))
}

/// Parse a canonical-table token into its [`CanonicalTable`].
///
/// The wire vocabulary the whole rule surface uses (plural `records`/`readings`,
/// matching the UI `CanonicalTable` type) — distinct from the singular SurrealDB
/// `register_name`s [`CanonicalTable::parse`] resolves.
pub(crate) fn parse_table(raw: &str) -> Result<CanonicalTable, String> {
    match raw {
        "records" => Ok(CanonicalTable::Records),
        "readings" => Ok(CanonicalTable::Readings),
        "tags" => Ok(CanonicalTable::Tags),
        "audit" => Ok(CanonicalTable::Audit),
        "insights" => Ok(CanonicalTable::Insights),
        "trace_summary" => Ok(CanonicalTable::TraceSummary),
        other => Err(format!("unknown table `{other}`")),
    }
}

/// Parse a grain token into its [`Grain`].
fn parse_grain(raw: &str) -> Result<Grain, String> {
    match raw {
        "minute" => Ok(Grain::Minute),
        "hour" => Ok(Grain::Hour),
        "day" => Ok(Grain::Day),
        "week" => Ok(Grain::Week),
        other => Err(format!("unknown grain `{other}`")),
    }
}

/// Parse an aggregate token into its [`Aggregate`].
fn parse_aggregate(raw: &str) -> Result<Aggregate, String> {
    match raw {
        "avg" => Ok(Aggregate::Avg),
        "min" => Ok(Aggregate::Min),
        "max" => Ok(Aggregate::Max),
        "sum" => Ok(Aggregate::Sum),
        "count" => Ok(Aggregate::Count),
        "first" => Ok(Aggregate::First),
        "last" => Ok(Aggregate::Last),
        other => Err(format!("unknown aggregate `{other}`")),
    }
}

/// A rule as returned to a client — the full definition plus storage metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RuleDto {
    /// The record id the rule is stored under (its delete/dry-run handle).
    pub id: String,
    /// The rule's stable name — its composition handle and the audited target.
    pub name: String,
    /// The Rhai script that produces the decision from the bound window values.
    pub script: String,
    /// The window-value inputs the script reads.
    #[serde(default)]
    pub inputs: Vec<BindingDto>,
    /// The names of the sub-rules this script may `invoke` (the composition set).
    #[serde(default)]
    pub subrules: Vec<String>,
    /// The insight kind this rule's decision is recorded and published under.
    pub output: String,
    /// When the rule was created (RFC 3339, UTC).
    pub created: String,
    /// When the rule was last updated (RFC 3339, UTC).
    pub updated: String,
}

impl RuleDto {
    /// Reconstruct a [`RuleDto`] from a stored `kind:"rule"` record, or `None` if
    /// the content is not a well-formed rule document.
    #[must_use]
    pub fn from_record(record: Record) -> Option<Self> {
        let doc: RuleDoc = serde_json::from_value(record.content).ok()?;
        Some(Self {
            id: record.id.to_string(),
            name: doc.name,
            script: doc.script,
            inputs: doc.inputs,
            subrules: doc.subrules,
            output: doc.output,
            created: record.created.to_string(),
            updated: record.updated.to_string(),
        })
    }
}

/// The persisted rule document — the `content` of a `kind:"rule"` record.
///
/// Deserialised from a stored record to project a [`RuleDto`] and to reconstruct
/// a [`rubix_rules::Rule`] for a dry-run; serialised from a create/update request
/// to write the record's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDoc {
    /// The discriminant the record list filters on (`kind:"rule"`).
    pub kind: String,
    /// The rule's stable name.
    pub name: String,
    /// The Rhai script.
    pub script: String,
    /// The window-value inputs.
    #[serde(default)]
    pub inputs: Vec<BindingDto>,
    /// The sub-rules this rule composes by name.
    #[serde(default)]
    pub subrules: Vec<String>,
    /// The insight kind this rule emits.
    pub output: String,
}

/// The body of a create-rule request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateRuleRequest {
    /// The rule's stable name (a lowercase slug, validated server-side).
    pub name: String,
    /// The Rhai script.
    pub script: String,
    /// The window-value inputs.
    #[serde(default)]
    pub inputs: Vec<BindingDto>,
    /// The sub-rules this rule composes by name.
    #[serde(default)]
    pub subrules: Vec<String>,
    /// The insight kind this rule emits.
    pub output: String,
}

/// The body of an update-rule request — replaces the rule's definition.
///
/// The name is immutable (it is the composition handle other rules reference), so
/// it is not part of the body; the path id addresses the rule.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateRuleRequest {
    /// The Rhai script.
    pub script: String,
    /// The window-value inputs.
    #[serde(default)]
    pub inputs: Vec<BindingDto>,
    /// The sub-rules this rule composes by name.
    #[serde(default)]
    pub subrules: Vec<String>,
    /// The insight kind this rule emits.
    pub output: String,
}

/// The body of a dry-run request — run an inline draft against real history.
///
/// A debugger dry-runs the *on-screen* draft, which may be unsaved, so the script
/// and inputs travel in the body rather than being read from storage. `subrules`
/// names already-stored rules the draft composes — they are loaded into the
/// dry-run registry from the principal's scoped session.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DryRunRequest {
    /// The draft Rhai script to run.
    pub script: String,
    /// The draft's window-value inputs.
    #[serde(default)]
    pub inputs: Vec<BindingDto>,
    /// The stored sub-rules this draft composes by name.
    #[serde(default)]
    pub subrules: Vec<String>,
}

/// One window bucket as charted by the debugger — the frame a binding saw.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BucketDto {
    /// The bucket's epoch-aligned start (epoch microseconds).
    pub bucket_start: i64,
    /// Mean of the bucket's values.
    pub avg: f64,
    /// Smallest value in the bucket.
    pub min: f64,
    /// Largest value in the bucket.
    pub max: f64,
    /// Sum of the bucket's values.
    pub sum: f64,
    /// Number of samples in the bucket.
    pub count: u64,
    /// The earliest sample's value in the bucket.
    pub first: f64,
    /// The latest sample's value in the bucket.
    pub last: f64,
}

/// One resolved input in a dry-run: the buckets it saw and the value it selected.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ResolvedInputDto {
    /// The script variable name this input bound to.
    pub name: String,
    /// The window buckets the binding rolled up, ascending by start.
    pub buckets: Vec<BucketDto>,
    /// The aggregate value selected from the latest bucket — what the script read.
    pub value: f64,
}

/// One narrowing key a binding can scope its series by, with the distinct values
/// observed for it — the picker behind `filter_field` / `filter_value`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FilterFacetDto {
    /// The key a binding's `filter_field` would take (`series`, `measure`, …).
    pub key: String,
    /// The distinct values observed for `key`, sorted, capped server-side.
    pub values: Vec<String>,
    /// Whether more distinct values exist than were returned — the list is a
    /// sample, so the author may still need to type an unlisted value.
    pub truncated: bool,
}

/// What a canonical table offers a binding: the numeric series it exposes and the
/// keys those series can be narrowed by, discovered from the principal's visible
/// rows. Backs the studio's field/filter pickers so a binding is built from what
/// the backend actually holds rather than typed blind.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CatalogResponse {
    /// The table the facets were discovered for (echoed back).
    pub table: String,
    /// The fields a binding's `field` can take, sorted.
    pub fields: Vec<String>,
    /// The keys a binding's `filter_field` can take, each with its values.
    pub filters: Vec<FilterFacetDto>,
}

/// The verdict of a dry-run: the decision and the frame it decided on.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DryRunResponse {
    /// Whether the rule fired.
    pub fired: bool,
    /// The numeric value the decision turned on.
    pub value: f64,
    /// The short, deterministic reason the script produced.
    pub reason: String,
    /// The resolved window for each binding, in declaration order.
    pub inputs: Vec<ResolvedInputDto>,
}
