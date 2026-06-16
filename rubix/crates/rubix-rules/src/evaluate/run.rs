//! Execute one rule's Rhai script against its resolved inputs and child values.
//!
//! This is the moment Rhai owns the decision (`rubix/STACK-DEISGN.md`): given the
//! window values resolved for the rule's [`Binding`](crate::rule::Binding)s and
//! the decision values of any sub-rules already evaluated, compile the script and
//! run it to a [`Decision`]. The script reads each input by its binding name and
//! calls `invoke(id)` to read a child's value; both are wired through the engine
//! [`build_engine`] and the scope here. No I/O happens in this step — the window
//! values and child values are resolved upstream — so script execution is
//! deterministic and offline.

use std::collections::HashMap;

use rhai::{Dynamic, Scope};

use crate::engine::{Decision, build_engine, from_dynamic};
use crate::error::{Result, RuleError};
use crate::rule::Rule;

/// Run `rule`'s script with `inputs` bound by name and `child_values` exposed to
/// `invoke`.
///
/// `inputs` maps each binding name to its resolved window value; `child_values`
/// maps each declared sub-rule id to the decision value it produced. Returns the
/// [`Decision`] the script computed.
///
/// # Errors
/// Returns [`RuleError::Compile`] if the script does not compile, or
/// [`RuleError::Evaluate`] if it fails at runtime or returns a non-decision
/// value.
pub fn run_script(
    rule: &Rule,
    inputs: &HashMap<String, f64>,
    child_values: HashMap<String, f64>,
) -> Result<Decision> {
    let engine = build_engine(child_values);
    let ast = engine
        .compile(&rule.script)
        .map_err(|e| RuleError::Compile(e.to_string()))?;

    let mut scope = Scope::new();
    for (name, value) in inputs {
        scope.push(name.clone(), *value);
    }

    let result: Dynamic = engine
        .eval_ast_with_scope(&mut scope, &ast)
        .map_err(|e| RuleError::Evaluate(e.to_string()))?;
    from_dynamic(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rubix_core::Id;
    use rubix_query::{CanonicalTable, Grain};

    use crate::rule::{Aggregate, Binding, Rule};

    use super::run_script;

    fn temp_rule(script: &str) -> Rule {
        Rule::new(
            Id::from_raw("r"),
            script,
            vec![Binding::new(
                "temp",
                CanonicalTable::Records,
                "temperature",
                Grain::Minute,
                Aggregate::Avg,
            )],
            "high-temp",
        )
    }

    #[test]
    fn a_bound_input_drives_the_decision() {
        let rule = temp_rule("temp > 25.0");
        let mut inputs = HashMap::new();
        inputs.insert("temp".to_owned(), 30.0_f64);
        let decision = run_script(&rule, &inputs, HashMap::new()).unwrap();
        assert!(decision.fired);
    }

    #[test]
    fn a_below_threshold_input_does_not_fire() {
        let rule = temp_rule("temp > 25.0");
        let mut inputs = HashMap::new();
        inputs.insert("temp".to_owned(), 10.0_f64);
        let decision = run_script(&rule, &inputs, HashMap::new()).unwrap();
        assert!(!decision.fired);
    }

    #[test]
    fn a_script_can_return_a_decision_map() {
        let rule = temp_rule(r#"#{ fired: temp > 25.0, value: temp, reason: "avg over window" }"#);
        let mut inputs = HashMap::new();
        inputs.insert("temp".to_owned(), 28.0_f64);
        let decision = run_script(&rule, &inputs, HashMap::new()).unwrap();
        assert!(decision.fired);
        assert_eq!(decision.value, 28.0);
        assert_eq!(decision.reason, "avg over window");
    }

    #[test]
    fn invoke_reads_a_child_value() {
        let rule = Rule::new(
            Id::from_raw("parent"),
            r#"invoke("child") > 0.5"#,
            Vec::new(),
            "out",
        )
        .composing(vec![Id::from_raw("child")]);
        let mut children = HashMap::new();
        children.insert("child".to_owned(), 1.0_f64);
        let decision = run_script(&rule, &HashMap::new(), children).unwrap();
        assert!(decision.fired);
    }

    #[test]
    fn a_broken_script_is_a_compile_error() {
        let rule = temp_rule("temp >");
        let err = run_script(&rule, &HashMap::new(), HashMap::new()).unwrap_err();
        assert!(matches!(err, crate::error::RuleError::Compile(_)));
    }
}
