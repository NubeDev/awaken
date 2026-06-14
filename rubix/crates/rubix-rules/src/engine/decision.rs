//! The decision a rule's script produces — the thing Rhai owns.
//!
//! Rhai owns the *decision* (`rubix/STACK-DEISGN.md`); a [`Decision`] is its
//! output: whether the rule fired, the numeric value it decided on, and a
//! human-readable reason. It is recorded as the insight content, published on the
//! data-change event, and stamped into the evaluation's root span — one decision,
//! three sinks, one correlation id. A script returns either a bare `bool` (fired,
//! with the value defaulting to `1.0`/`0.0`) or a Rhai map carrying `fired`,
//! `value`, and `reason`; [`from_dynamic`] normalises both into this type.

use rhai::Dynamic;

use crate::error::{Result, RuleError};

/// A rule's decision: did it fire, on what value, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// Whether the rule fired.
    pub fired: bool,
    /// The numeric value the decision turned on (e.g. the observed average).
    pub value: f64,
    /// A short, deterministic explanation of the decision.
    pub reason: String,
}

impl Decision {
    /// Render this decision as the insight content JSON the gate records.
    #[must_use]
    pub fn to_content(&self, output: &str) -> serde_json::Value {
        serde_json::json!({
            "kind": output,
            "fired": self.fired,
            "value": self.value,
            "reason": self.reason,
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
    Ok(Decision {
        fired,
        value: decided,
        reason,
    })
}

/// Map a Rhai type-mismatch into an evaluation error.
fn type_error(actual: &'static str) -> RuleError {
    RuleError::Evaluate(format!("decision field had unexpected type: {actual}"))
}

#[cfg(test)]
mod tests {
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
            }
        );
    }

    #[test]
    fn a_non_decision_return_is_an_error() {
        assert!(from_dynamic(Dynamic::from(7_i64)).is_err());
    }

    #[test]
    fn content_carries_the_kind_and_fields() {
        let decision = Decision {
            fired: true,
            value: 30.0,
            reason: "hot".to_owned(),
        };
        let content = decision.to_content("high-temp");
        assert_eq!(content["kind"], "high-temp");
        assert_eq!(content["fired"], true);
        assert_eq!(content["value"], 30.0);
        assert_eq!(content["reason"], "hot");
    }
}
