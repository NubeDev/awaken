//! The `Rule` — a composable, offline rule definition.
//!
//! A rule is the deterministic decision unit (`rubix/docs/SCOPE.md`, "Rhai —
//! rules and insights"): a Rhai script, the input [`Binding`]s that resolve its
//! window values from DataFusion, and the output insight shape it produces. Rules
//! are composable — a script invokes another rule by id (see
//! [`compose`](crate::evaluate)) — and fire offline with no cloud dependency.
//! The rule definition itself holds no engine or session: it is data, evaluated
//! by the [`evaluate`](crate::evaluate) pipeline that owns the I/O.

use rubix_core::Id;

use super::bind::Binding;

/// A composable rule: script + input bindings + output insight shape.
///
/// The `id` names the rule for composition and is the audited target when its
/// insight is recorded. The `script` is Rhai source compiled once per evaluation;
/// it reads each binding's resolved value by [`Binding::name`] and returns the
/// decision. `subrules` declares the rules this script may `invoke` — the
/// composition dependency set, resolved depth-first before the script runs so the
/// dependency graph is explicit and fail-closed (a script cannot invoke a rule it
/// did not declare). `output` is the insight kind stamped onto the recorded
/// insight and the published data-change event, so a downstream subscriber can
/// filter by it.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Stable identifier — the composition handle and the audited insight target.
    pub id: Id,
    /// The Rhai script that produces the decision from the bound window values.
    pub script: String,
    /// The window values bound into the script before it runs.
    pub inputs: Vec<Binding>,
    /// The ids of the sub-rules this script may `invoke` (the composition set).
    pub subrules: Vec<Id>,
    /// The insight kind this rule's decision is recorded and published under.
    pub output: String,
}

impl Rule {
    /// Define a leaf rule with `id`, `script`, input `inputs`, and `output` kind
    /// (no sub-rules).
    #[must_use]
    pub fn new(
        id: Id,
        script: impl Into<String>,
        inputs: Vec<Binding>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            id,
            script: script.into(),
            inputs,
            subrules: Vec::new(),
            output: output.into(),
        }
    }

    /// Declare the sub-rules this rule composes, returning the updated rule.
    ///
    /// The script may `invoke` exactly these ids; the evaluation pipeline
    /// pre-evaluates each before running this rule's script.
    #[must_use]
    pub fn composing(mut self, subrules: Vec<Id>) -> Self {
        self.subrules = subrules;
        self
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::Id;
    use rubix_query::{CanonicalTable, Grain};

    use crate::rule::bind::{Aggregate, Binding};

    use super::Rule;

    #[test]
    fn rule_carries_script_inputs_and_output() {
        let rule = Rule::new(
            Id::from_raw("high-temp"),
            "temp > 25.0",
            vec![Binding::new(
                "temp",
                CanonicalTable::Records,
                "temperature",
                Grain::Minute,
                Aggregate::Avg,
            )],
            "high-temperature",
        );
        assert_eq!(rule.id.as_str(), "high-temp");
        assert_eq!(rule.inputs.len(), 1);
        assert_eq!(rule.output, "high-temperature");
    }
}
