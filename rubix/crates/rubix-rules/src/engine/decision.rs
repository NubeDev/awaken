//! The decision a rule's script produces — the thing Rhai owns.
//!
//! Rhai owns the *decision* (`rubix/STACK-DEISGN.md`); a [`Decision`] is its
//! output: whether the rule fired, the numeric value it decided on, and a
//! human-readable reason. It is recorded as the insight content, published on the
//! data-change event, and stamped into the evaluation's root span — one decision,
//! three sinks, one correlation id. A script returns either a bare `bool` (fired,
//! with the value defaulting to `1.0`/`0.0`) or a Rhai map carrying `fired`,
//! `value`, and `reason`; [`from_dynamic`] normalises both into this type.

use std::collections::BTreeMap;

use rhai::Dynamic;

use crate::error::{Result, RuleError};

/// A rule's decision: did it fire, on what value, why — and, as an evaluation,
/// the scores it produced and the group those scores compare within.
///
/// `scores` + `group_id` are the §5c lift (`rubix/docs/design/LAMINAR-BORROW.md`):
/// a `scores: map<string,f64>` turns a rule firing into a comparable, chartable
/// *evaluation datapoint*, and `group_id` ties every run of the same evaluation
/// together so they can be compared over time. Both are optional — a plain
/// threshold rule produces no scores and leaves the group to default to the rule's
/// own identity — so existing rules are unaffected.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// Whether the rule fired.
    pub fired: bool,
    /// The numeric value the decision turned on (e.g. the observed average).
    pub value: f64,
    /// A short, deterministic explanation of the decision.
    pub reason: String,
    /// Named scores this evaluation produced (empty for a non-scoring rule).
    /// Ordered (`BTreeMap`) so the recorded content is deterministic.
    pub scores: BTreeMap<String, f64>,
    /// The evaluation group these scores compare within; `None` falls back to the
    /// rule's identity when recorded (see [`Decision::to_content`]).
    pub group_id: Option<String>,
}

impl Decision {
    /// Render this decision as the insight content JSON the gate records.
    ///
    /// `group` is the effective evaluation group — the caller passes the rule's
    /// identity as the fallback used when the decision declared no `group_id`, so
    /// every recorded insight is groupable for cross-run comparison (§5c).
    #[must_use]
    pub fn to_content(&self, output: &str, group: &str) -> serde_json::Value {
        let group_id = self.group_id.as_deref().unwrap_or(group);
        serde_json::json!({
            "kind": output,
            "fired": self.fired,
            "value": self.value,
            "reason": self.reason,
            "scores": self.scores,
            "group_id": group_id,
        })
    }
}

/// Normalise a script's returned Rhai value into a [`Decision`].
///
/// Accepts a bare `bool` (fired; value `1.0` when true, `0.0` when false, reason
/// derived) or a map with `fired` (bool), `value` (float, optional), and `reason`
/// (string, optional). Any other return shape is a [`RuleError::Evaluate`] — a
/// script must produce a decision, never an ambiguous value the runtime would
/// have to guess at (CLAUDE.md "Core Rules": no fallbacks that hide failure).
///
/// # Errors
/// Returns [`RuleError::Evaluate`] if `value` is neither a bool nor a decision
/// map, or if a present field has the wrong type.
pub fn from_dynamic(value: Dynamic) -> Result<Decision> {
    if value.is_bool() {
        let fired = value.as_bool().map_err(type_error)?;
        return Ok(Decision {
            fired,
            value: if fired { 1.0 } else { 0.0 },
            reason: if fired { "fired".to_owned() } else { "not fired".to_owned() },
            scores: BTreeMap::new(),
            group_id: None,
        });
    }
    if value.is_map() {
        return from_map(value);
    }
    Err(RuleError::Evaluate(format!(
        "script must return a bool or a decision map, got {}",
        value.type_name()
    )))
}

/// Build a decision from a Rhai map return value.
fn from_map(value: Dynamic) -> Result<Decision> {
    let map = value.cast::<rhai::Map>();
    let fired = map
        .get("fired")
        .ok_or_else(|| RuleError::Evaluate("decision map missing 'fired'".to_owned()))?
        .as_bool()
        .map_err(type_error)?;
    let decided = match map.get("value") {
        Some(v) => v.as_float().map_err(type_error)?,
        None => {
            if fired {
                1.0
            } else {
                0.0
            }
        }
    };
    let reason = match map.get("reason") {
        Some(v) => v.clone().into_string().map_err(type_error)?,
        None => String::new(),
    };
    let scores = match map.get("scores") {
        Some(v) => scores_from_dynamic(v)?,
        None => BTreeMap::new(),
    };
    let group_id = match map.get("group_id") {
        Some(v) => Some(v.clone().into_string().map_err(type_error)?),
        None => None,
    };
    Ok(Decision {
        fired,
        value: decided,
        reason,
        scores,
        group_id,
    })
}

/// Parse a `scores` field — a Rhai map of name → float — into an ordered map.
///
/// Each value must be a float (or integer, coerced); a non-map `scores` or a
/// non-numeric score is a [`RuleError::Evaluate`], never a silently dropped one
/// (CLAUDE.md "Core Rules": no fallbacks that hide failure).
fn scores_from_dynamic(value: &Dynamic) -> Result<BTreeMap<String, f64>> {
    if !value.is_map() {
        return Err(RuleError::Evaluate(format!(
            "decision 'scores' must be a map, got {}",
            value.type_name()
        )));
    }
    let map = value.clone().cast::<rhai::Map>();
    let mut scores = BTreeMap::new();
    for (name, score) in map {
        let n = score.as_float().map_err(type_error)?;
        scores.insert(name.to_string(), n);
    }
    Ok(scores)
}

/// Map a Rhai type-mismatch into an evaluation error.
fn type_error(actual: &'static str) -> RuleError {
    RuleError::Evaluate(format!("decision field had unexpected type: {actual}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rhai::Dynamic;

    use super::{from_dynamic, Decision};

    #[test]
    fn a_bare_true_fires_with_value_one() {
        let decision = from_dynamic(Dynamic::from(true)).unwrap();
        assert!(decision.fired);
        assert_eq!(decision.value, 1.0);
    }

    #[test]
    fn a_bare_false_does_not_fire() {
        let decision = from_dynamic(Dynamic::from(false)).unwrap();
        assert!(!decision.fired);
        assert_eq!(decision.value, 0.0);
    }

    #[test]
    fn a_map_carries_value_and_reason() {
        let mut map = rhai::Map::new();
        map.insert("fired".into(), Dynamic::from(true));
        map.insert("value".into(), Dynamic::from(42.0_f64));
        map.insert("reason".into(), Dynamic::from("too hot".to_string()));
        let decision = from_dynamic(Dynamic::from_map(map)).unwrap();
        assert_eq!(
            decision,
            Decision {
                fired: true,
                value: 42.0,
                reason: "too hot".to_owned(),
                scores: BTreeMap::new(),
                group_id: None,
            }
        );
    }

    #[test]
    fn a_map_carries_scores_and_group_id() {
        let mut scores = rhai::Map::new();
        scores.insert("groundedness".into(), Dynamic::from(0.9_f64));
        scores.insert("relevance".into(), Dynamic::from(0.75_f64));
        let mut map = rhai::Map::new();
        map.insert("fired".into(), Dynamic::from(true));
        map.insert("scores".into(), Dynamic::from_map(scores));
        map.insert("group_id".into(), Dynamic::from("qa-suite".to_string()));
        let decision = from_dynamic(Dynamic::from_map(map)).unwrap();
        assert_eq!(decision.scores.get("groundedness"), Some(&0.9));
        assert_eq!(decision.scores.get("relevance"), Some(&0.75));
        assert_eq!(decision.group_id.as_deref(), Some("qa-suite"));
    }

    #[test]
    fn a_non_map_scores_field_is_an_error() {
        let mut map = rhai::Map::new();
        map.insert("fired".into(), Dynamic::from(true));
        map.insert("scores".into(), Dynamic::from(7_i64));
        assert!(from_dynamic(Dynamic::from_map(map)).is_err());
    }

    #[test]
    fn a_non_decision_return_is_an_error() {
        assert!(from_dynamic(Dynamic::from(7_i64)).is_err());
    }

    #[test]
    fn content_carries_the_kind_and_fields() {
        let mut scores = BTreeMap::new();
        scores.insert("groundedness".to_owned(), 0.9_f64);
        let decision = Decision {
            fired: true,
            value: 30.0,
            reason: "hot".to_owned(),
            scores,
            group_id: None,
        };
        let content = decision.to_content("high-temp", "rule-7");
        assert_eq!(content["kind"], "high-temp");
        assert_eq!(content["scores"]["groundedness"], 0.9);
        // group_id falls back to the rule identity when the decision declared none.
        assert_eq!(content["group_id"], "rule-7");
        assert_eq!(content["fired"], true);
        assert_eq!(content["value"], 30.0);
        assert_eq!(content["reason"], "hot");
    }
}
