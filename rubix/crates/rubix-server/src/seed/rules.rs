//! Demo rules: ten rules per tenant, simple threshold → composed → scored.
//!
//! A rule is the deterministic decision unit (`rubix/docs/SCOPE.md`, "Rhai —
//! rules and insights"); the studio (`/rules`) is where a user authors and
//! dry-runs them. So the seed plants a worked set against the demo readings, going
//! from the simplest shape to the most complex, so opening the studio shows *how
//! rules work* rather than an empty list:
//!
//! - **thresholds** — one binding, one comparison (high temperature, elevated CO₂).
//! - **multi-input** — several bindings compared (temp vs. its setpoint, a voltage
//!   band).
//! - **composition** — a rule that `invoke`s other rules (comfort risk = hot AND
//!   stuffy; a building alert that ORs the composed risks).
//! - **scoring** — a rule that emits a `scores` map, the §5c evaluation-point shape
//!   a dashboard can compare over time.
//!
//! Every binding reads the shared numeric `content.value` narrowed to one
//! `measure` (the readings store every metric at `value`), so each rule decides on
//! a real, distinct series. Rules persist as `kind:"rule"` records written through
//! the WS-05 gate as the tenant operator — the same path the studio writes them —
//! so they carry real audit rows and read back on the scoped session.

use rubix_core::{Id, Principal};
use rubix_gate::{Capability, Change, Command, apply};
use serde_json::{Value, json};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::SeedError;

/// One demo rule's definition, in the `kind:"rule"` content shape the studio uses.
struct RuleSpec {
    /// The rule name — its composition handle and the list label.
    name: &'static str,
    /// The Rhai script: reads each input by name, returns a bool or a decision map.
    script: &'static str,
    /// The window-value inputs, each a `(name, measure, grain, aggregate)` over the
    /// shared `content.value` series narrowed to that `measure`.
    inputs: &'static [Input],
    /// The sub-rules this rule composes by name (resolved by the engine).
    subrules: &'static [&'static str],
    /// The insight kind the rule emits.
    output: &'static str,
}

/// A binding spelled out for the seed: a script variable bound to the `aggregate`
/// of the `measure`'s readings at `grain`.
struct Input {
    /// The script variable name.
    name: &'static str,
    /// The `content.measure` the `content.value` series is narrowed to.
    measure: &'static str,
    /// The bucket grain.
    grain: &'static str,
    /// The bucket aggregate.
    aggregate: &'static str,
}

/// The ten demo rules, ordered simplest to most complex.
const RULES: &[RuleSpec] = &[
    // 1 — the simplest shape: one binding, one threshold. Zone temp runs 22 ± 2,
    // so a >24 average flags the warm end of the band. `value` is the *fired*
    // indicator (1/0), not the magnitude, so a composing rule's `invoke(...) > 0.5`
    // reads cleanly; the magnitude rides the reason string.
    RuleSpec {
        name: "high-zone-temp",
        script: r#"let hot = temp > 24.0; #{ fired: hot, value: if hot { 1.0 } else { 0.0 }, reason: "zone temperature " + temp + "°C (limit 24)" }"#,
        inputs: &[Input { name: "temp", measure: "temp", grain: "hour", aggregate: "avg" }],
        subrules: &[],
        output: "high-temperature",
    },
    // 2 — another threshold, different metric. CO₂ runs 600 ± 150; >700 is stuffy.
    RuleSpec {
        name: "co2-elevated",
        script: r#"let high = co2 > 700.0; #{ fired: high, value: if high { 1.0 } else { 0.0 }, reason: "CO₂ " + co2 + " ppm (limit 700)" }"#,
        inputs: &[Input { name: "co2", measure: "co2", grain: "hour", aggregate: "avg" }],
        subrules: &[],
        output: "air-quality",
    },
    // 3 — a threshold on the *minimum* of the window, not the average: catches a
    // pressure dip even if the average looks fine. Supply pressure runs 3 ± 0.5.
    RuleSpec {
        name: "low-water-pressure",
        script: r#"#{ fired: p_min < 2.6, value: p_min, reason: "supply pressure dipped below 2.6 bar" }"#,
        inputs: &[Input { name: "p_min", measure: "pressure", grain: "hour", aggregate: "min" }],
        subrules: &[],
        output: "water-pressure",
    },
    // 4 — two bindings off the same metric (min and max) for a *band* check: line
    // voltage should stay within ±3% of 230 V.
    RuleSpec {
        name: "voltage-out-of-band",
        script: r#"#{
            fired: v_min < 223.0 || v_max > 237.0,
            value: v_max,
            reason: "line voltage left the 223–237 V band"
        }"#,
        inputs: &[
            Input { name: "v_min", measure: "voltage", grain: "hour", aggregate: "min" },
            Input { name: "v_max", measure: "voltage", grain: "hour", aggregate: "max" },
        ],
        subrules: &[],
        output: "power-quality",
    },
    // 5 — a peak-demand threshold on the max. Active power runs 120 ± 40 kW.
    // `value` is the fired indicator (1/0) so building-alert can compose it.
    RuleSpec {
        name: "high-power-draw",
        script: r#"let peak = kw_peak > 150.0; #{ fired: peak, value: if peak { 1.0 } else { 0.0 }, reason: "peak power " + kw_peak + " kW (limit 150)" }"#,
        inputs: &[Input { name: "kw_peak", measure: "kw", grain: "hour", aggregate: "max" }],
        subrules: &[],
        output: "energy-demand",
    },
    // 6 — a wide-open damper suggests the loop is fighting to hold setpoint.
    RuleSpec {
        name: "damper-wide-open",
        script: r#"#{ fired: damper > 70.0, value: damper, reason: "damper averaging over 70% open" }"#,
        inputs: &[Input { name: "damper", measure: "damper", grain: "hour", aggregate: "avg" }],
        subrules: &[],
        output: "hvac-control",
    },
    // 7 — two *different* metrics compared: zone temp drifting above its own
    // setpoint by more than 1.5°C is a control-deviation signal, not a fixed limit.
    RuleSpec {
        name: "temp-above-setpoint",
        script: r#"#{
            fired: temp > sp + 1.5,
            value: temp - sp,
            reason: "zone is more than 1.5°C above setpoint"
        }"#,
        inputs: &[
            Input { name: "temp", measure: "temp", grain: "hour", aggregate: "avg" },
            Input { name: "sp", measure: "setpoint", grain: "hour", aggregate: "avg" },
        ],
        subrules: &[],
        output: "control-deviation",
    },
    // 8 — composition: a rule that fires only when two *other* rules both fire.
    // `invoke(name)` returns a sub-rule's decision value — the leaves above set
    // that to 1.0 when fired and 0.0 otherwise, so `> 0.5` reads as "did it fire".
    // `value` is itself a 1/0 indicator so a higher-level rule can compose this one.
    RuleSpec {
        name: "comfort-risk",
        script: r#"let risk = invoke("high-zone-temp") > 0.5 && invoke("co2-elevated") > 0.5; #{ fired: risk, value: if risk { 1.0 } else { 0.0 }, reason: "hot AND stuffy — occupant comfort at risk" }"#,
        inputs: &[],
        subrules: &["high-zone-temp", "co2-elevated"],
        output: "comfort",
    },
    // 9 — a scored evaluation (§5c): instead of one boolean, emit a `scores` map of
    // sub-metrics plus a `group_id`, so each run is a comparable datapoint a
    // dashboard can trend. Fires when the blended health score is poor.
    RuleSpec {
        name: "hvac-health-score",
        script: r#"
            let temp_ok = if temp <= 24.0 { 1.0 } else { 0.0 };
            let air_ok = if co2 <= 700.0 { 1.0 } else { 0.0 };
            let damper_ok = if damper <= 70.0 { 1.0 } else { 0.0 };
            let score = (temp_ok + air_ok + damper_ok) / 3.0;
            #{
                fired: score < 1.0,
                value: score,
                reason: "HVAC health score",
                group_id: "hvac-health",
                scores: #{ temperature: temp_ok, air: air_ok, damper: damper_ok, overall: score }
            }
        "#,
        inputs: &[
            Input { name: "temp", measure: "temp", grain: "hour", aggregate: "avg" },
            Input { name: "co2", measure: "co2", grain: "hour", aggregate: "avg" },
            Input { name: "damper", measure: "damper", grain: "hour", aggregate: "avg" },
        ],
        subrules: &[],
        output: "hvac-health",
    },
    // 10 — composition of a composed rule: a top-level building alert that ORs the
    // comfort risk (itself composed) with the peak-power signal. Shows a rule tree
    // two levels deep — the "why did this fire" the trace view reassembles.
    RuleSpec {
        name: "building-alert",
        script: r#"let alert = invoke("comfort-risk") > 0.5 || invoke("high-power-draw") > 0.5; #{ fired: alert, value: if alert { 1.0 } else { 0.0 }, reason: "building needs attention — comfort risk or peak demand" }"#,
        inputs: &[],
        subrules: &["comfort-risk", "high-power-draw"],
        output: "building-alert",
    },
];

/// Write the ten demo rules for one tenant as `operator`, returning the count.
///
/// Each rule is a `kind:"rule"` record created through the gate (so it audits and
/// reads back like a studio-authored rule). Ids are namespace-prefixed so two
/// tenants' identically-named rules do not collide in the shared `record` table.
///
/// # Errors
/// Returns a [`SeedError`] if any gate write fails.
pub async fn seed_rules(
    db: &Surreal<Db>,
    namespace: &str,
    operator: &Principal,
) -> Result<usize, SeedError> {
    for spec in RULES {
        let id = Id::from_raw(format!("{namespace}--rule--{}", spec.name));
        put(db, operator, &id, rule_content(spec)).await?;
    }
    Ok(RULES.len())
}

/// Build the `kind:"rule"` content for one spec — the exact shape `RuleDoc`
/// deserialises and the studio writes.
fn rule_content(spec: &RuleSpec) -> Value {
    let inputs: Vec<Value> = spec
        .inputs
        .iter()
        .map(|i| {
            json!({
                "name": i.name,
                "table": "records",
                "field": "value",
                "grain": i.grain,
                "aggregate": i.aggregate,
                "filter_field": "measure",
                "filter_value": i.measure,
            })
        })
        .collect();
    json!({
        "kind": "rule",
        "name": spec.name,
        "script": spec.script,
        "inputs": inputs,
        "subrules": spec.subrules,
        "output": spec.output,
    })
}

/// Create `content` at `target` through the gate as the tenant operator.
///
/// A rule write is a definition mutation, gated on
/// [`RuleDefine`](rubix_gate::Capability::RuleDefine) — the same capability the
/// transport routes a rule create through.
async fn put(
    db: &Surreal<Db>,
    operator: &Principal,
    target: &Id,
    content: Value,
) -> Result<(), SeedError> {
    let command = Command::new(
        operator.clone(),
        Capability::RuleDefine,
        target.clone(),
        Change::Create(content),
    );
    apply(db, &command, None)
        .await
        .map(|_| ())
        .map_err(|e| SeedError::new("write rule", e))
}
