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
//! Every binding rolls up the `reading` table scoped to one **series** — the
//! point record a sample's `series` link points at, e.g.
//! `acme--hq--ahu-1--zone-temp` (`rubix/docs/design/READINGS-TIMESERIES.md`). The
//! reading's numeric `value` is rolled up over the window at the chosen grain, so
//! each rule decides on one real, distinct metric. Rules persist as `kind:"rule"`
//! records written through the WS-05 gate as the tenant operator — the same path
//! the studio writes them — so they carry real audit rows and read back on the
//! scoped session.
//!
//! Series ids are per-tenant (`{namespace}--{site}--{equip}--{point}`), so the
//! rules target each tenant's first site (acme → `hq`, globex → `tower`); the
//! seed builds the full id from the namespace at write time.

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
/// of one reading series at `grain`.
struct Input {
    /// The script variable name.
    name: &'static str,
    /// The series suffix (`equip--point`); the namespace + first site are prefixed
    /// at write time to form the full series id the binding filters on.
    series: &'static str,
    /// The bucket grain (`week` rolls the whole trailing window into one bucket).
    grain: &'static str,
    /// The bucket aggregate.
    aggregate: &'static str,
}

/// The ten demo rules, ordered simplest to most complex. Thresholds are tuned to
/// the seeded waves (temp ≈20–24, CO₂ ≈450–740, power ≈80–157 kW, voltage
/// ≈225–235, pressure ≈2.5–3.5, damper ≈20–78) so the set shows a mix of fired
/// and all-clear verdicts. Grain is `week`, which rolls the whole 24 h trailing
/// window into one bucket, so each aggregate reflects the entire series.
const RULES: &[RuleSpec] = &[
    // 1 — the simplest shape: one binding, one threshold. Peak zone temp over the
    // window above 23.5°C flags the warm end of the band. `value` is the *fired*
    // indicator (1/0), not the magnitude, so a composing rule's `invoke(...) > 0.5`
    // reads cleanly; the magnitude rides the reason string.
    RuleSpec {
        name: "high-zone-temp",
        script: r#"let hot = temp > 23.5; #{ fired: hot, value: if hot { 1.0 } else { 0.0 }, reason: "peak zone temperature " + temp + "°C (limit 23.5)" }"#,
        inputs: &[Input { name: "temp", series: "ahu-1--zone-temp", grain: "week", aggregate: "max" }],
        subrules: &[],
        output: "high-temperature",
    },
    // 2 — another threshold, different metric/series. Peak CO₂ over 700 ppm is stuffy.
    RuleSpec {
        name: "co2-elevated",
        script: r#"let high = co2 > 700.0; #{ fired: high, value: if high { 1.0 } else { 0.0 }, reason: "peak CO₂ " + co2 + " ppm (limit 700)" }"#,
        inputs: &[Input { name: "co2", series: "ahu-1--co2", grain: "week", aggregate: "max" }],
        subrules: &[],
        output: "air-quality",
    },
    // 3 — a threshold on the *minimum* of the window, not the average: catches a
    // pressure dip even if the average looks fine.
    RuleSpec {
        name: "low-water-pressure",
        script: r#"#{ fired: p_min < 2.6, value: p_min, reason: "supply pressure dipped to " + p_min + " bar (floor 2.6)" }"#,
        inputs: &[Input { name: "p_min", series: "water-main--pressure", grain: "week", aggregate: "min" }],
        subrules: &[],
        output: "water-pressure",
    },
    // 4 — two bindings off the same series (min and max) for a *band* check: line
    // voltage should stay within ±3% of 230 V. With the seeded wave it stays in
    // band, so this is the "all clear" example.
    RuleSpec {
        name: "voltage-out-of-band",
        script: r#"#{
            fired: v_min < 224.0 || v_max > 236.0,
            value: v_max,
            reason: "line voltage " + v_min + "–" + v_max + " V (band 224–236)"
        }"#,
        inputs: &[
            Input { name: "v_min", series: "elec-main--voltage", grain: "week", aggregate: "min" },
            Input { name: "v_max", series: "elec-main--voltage", grain: "week", aggregate: "max" },
        ],
        subrules: &[],
        output: "power-quality",
    },
    // 5 — a peak-demand threshold on the max. `value` is the fired indicator (1/0)
    // so building-alert can compose it.
    RuleSpec {
        name: "high-power-draw",
        script: r#"let peak = kw_peak > 150.0; #{ fired: peak, value: if peak { 1.0 } else { 0.0 }, reason: "peak power " + kw_peak + " kW (limit 150)" }"#,
        inputs: &[Input { name: "kw_peak", series: "elec-main--power", grain: "week", aggregate: "max" }],
        subrules: &[],
        output: "energy-demand",
    },
    // 6 — a wide-open damper suggests the loop is fighting to hold setpoint.
    RuleSpec {
        name: "damper-wide-open",
        script: r#"#{ fired: damper > 75.0, value: damper, reason: "damper peaked at " + damper + "% open (limit 75)" }"#,
        inputs: &[Input { name: "damper", series: "ahu-1--damper", grain: "week", aggregate: "max" }],
        subrules: &[],
        output: "hvac-control",
    },
    // 7 — two *different* series compared: zone temp drifting above its own setpoint
    // by more than 1.5°C is a control-deviation signal, not a fixed limit. The
    // averages track closely here, so this is a second "within control" example.
    RuleSpec {
        name: "temp-above-setpoint",
        script: r#"#{
            fired: temp > sp + 1.5,
            value: temp - sp,
            reason: "zone avg " + temp + "°C vs setpoint " + sp + "°C"
        }"#,
        inputs: &[
            Input { name: "temp", series: "ahu-1--zone-temp", grain: "week", aggregate: "avg" },
            Input { name: "sp", series: "ahu-1--setpoint", grain: "week", aggregate: "avg" },
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
    // dashboard can trend. Uses peak (max) of each series, and fires when the
    // blended health score is below perfect.
    RuleSpec {
        name: "hvac-health-score",
        script: r#"
            let temp_ok = if temp <= 23.5 { 1.0 } else { 0.0 };
            let air_ok = if co2 <= 700.0 { 1.0 } else { 0.0 };
            let damper_ok = if damper <= 75.0 { 1.0 } else { 0.0 };
            let score = (temp_ok + air_ok + damper_ok) / 3.0;
            #{
                fired: score < 1.0,
                value: score,
                reason: "HVAC health score " + score,
                group_id: "hvac-health",
                scores: #{ temperature: temp_ok, air: air_ok, damper: damper_ok, overall: score }
            }
        "#,
        inputs: &[
            Input { name: "temp", series: "ahu-1--zone-temp", grain: "week", aggregate: "max" },
            Input { name: "co2", series: "ahu-1--co2", grain: "week", aggregate: "max" },
            Input { name: "damper", series: "ahu-1--damper", grain: "week", aggregate: "max" },
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
    let site = first_site(namespace);
    for spec in RULES {
        let id = Id::from_raw(format!("{namespace}--rule--{}", spec.name));
        put(db, operator, &id, rule_content(spec, namespace, site)).await?;
    }
    Ok(RULES.len())
}

/// Seed one demo hook binding so the portfolio shows write-triggered rules.
///
/// Binds the simplest demo rule (`high-zone-temp`) to **updates of a `site`**: when
/// the operator edits the tenant's site record, the dispatcher re-evaluates the
/// temperature rule against the seeded readings and records a fresh insight
/// (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Server-side hooks"). A hook is a
/// `kind:"hook"` record like any other, written through the gate as the operator.
/// Returns the number of hooks seeded.
pub async fn seed_hooks(
    db: &Surreal<Db>,
    namespace: &str,
    operator: &Principal,
) -> Result<usize, SeedError> {
    let id = Id::from_raw(format!("{namespace}--hook--site-temp"));
    let content = json!({
        "kind": "hook",
        "match": "site",
        "on": ["update"],
        "rule": "high-zone-temp",
    });
    put(db, operator, &id, content).await?;
    Ok(1)
}

/// The first site key for a tenant — the site the demo rules target.
///
/// Kept in step with the portfolio topology (acme → `hq`, globex → `tower`); an
/// unknown namespace falls back to `hq` so the seed never panics.
fn first_site(namespace: &str) -> &'static str {
    match namespace {
        "globex" => "tower",
        _ => "hq",
    }
}

/// Build the `kind:"rule"` content for one spec — the exact shape `RuleDoc`
/// deserialises and the studio writes.
///
/// Each input becomes a reading binding: the numeric `value` of the `reading`
/// table rolled up over the window, scoped to the full series id
/// (`{namespace}--{site}--{suffix}`) via the `series` filter.
fn rule_content(spec: &RuleSpec, namespace: &str, site: &str) -> Value {
    let inputs: Vec<Value> = spec
        .inputs
        .iter()
        .map(|i| {
            json!({
                "name": i.name,
                "table": "readings",
                "field": "value",
                "grain": i.grain,
                "aggregate": i.aggregate,
                "filter_field": "series",
                "filter_value": format!("{namespace}--{site}--{}", i.series),
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
