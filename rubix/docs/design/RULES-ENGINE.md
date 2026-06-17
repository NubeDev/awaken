# Rules Engine — Rhai rules, decisions, insights & composition

Design for the **rule/insight runtime**: how a rule is defined, bound to time-window
inputs, evaluated deterministically in an embedded Rhai sandbox, composed from
sub-rules, and recorded as an audited insight through the gate. Reads against
[SCOPE.md](../SCOPE.md): *"Rules fire offline"* (the runtime is embedded, no cloud
dependency), *"Commands go through the gate; reads are SurrealDB-native"* (§7), and the
*"Two authz layers"* rule. The crate is [`rubix-rules`](../../crates/rubix-rules); its
HTTP surface (Rules Studio) lives in
[`rubix-server/src/http/rules/`](../../crates/rubix-server/src/http/rules).

This is the missing sibling of [HOOKS-AND-FILES.md](HOOKS-AND-FILES.md) (which fires
rules after a write) and [LAMINAR-BORROW.md](LAMINAR-BORROW.md) §5c (which maps a rule
firing onto the evaluation/trace model).

## The rule model

A rule is a record (`kind:"rule"`, no bespoke table) carrying a Rhai script plus the
declared inputs and sub-rules it may read. Domain type
[`Rule`](../../crates/rubix-rules/src/rule/define.rs):

| Field | Meaning |
| --- | --- |
| `id` / `name` | stable composition handle, unique per namespace |
| `script` | Rhai source |
| `inputs: Vec<Binding>` | window-value bindings the script reads as variables |
| `subrules: Vec<Id>` | the closure of rules this script may `invoke` |
| `output: String` | the `kind` of the insight a firing records |

Stored as a `RuleDoc` ([`dto/rule.rs`](../../crates/rubix-server/src/dto/rule.rs)) on
the generic `record` table. **Reads** run on the scoped session (row-level perms);
**mutations** cross the gate on `Capability::RuleDefine`.

### Time-window bindings

Each [`Binding`](../../crates/rubix-rules/src/rule/bind.rs) names a script variable and
how to compute its scalar value: a `CanonicalTable` (Readings / Records / Tags / Audit /
Insights / TraceSummary), a numeric `field`, a `Grain` (minute/hour/day/week), an
`Aggregate` (avg/min/max/sum/count/first/last), and an optional `(key, value)` filter to
narrow the series. Resolution pulls a DataFusion rollup at the declared grain over the
**scoped session**, takes the most recent bucket, and applies the aggregate. An empty
series is **fail-closed** (a binding error, not a silent zero).

## The Rhai sandbox

[`engine/register.rs`](../../crates/rubix-rules/src/engine/register.rs) hardens the
engine for deterministic, embedded evaluation: `MAX_OPERATIONS = 100_000`,
`MAX_CALL_LEVELS = 32`, and exactly **one** host function — `invoke(id) -> f64`, which
returns a pre-computed sub-rule's decision value (no I/O inside the script; child values
are resolved in Rust first). An unknown sub-rule id throws at runtime (fail-closed).

A script returns either a bare `bool` or a map. The map is normalized into a
[`Decision`](../../crates/rubix-rules/src/engine/decision.rs):

| Field | Source |
| --- | --- |
| `fired: bool` | required |
| `value: f64` | optional (bare bool → 1.0/0.0) |
| `reason: String` | optional, deterministic explanation |
| `scores: BTreeMap<String,f64>` | optional — the §5c evaluation lift |
| `group_id: Option<String>` | optional — cross-run grouping; falls back to the rule id |

Missing required fields or wrong types are an evaluate error, never a default.

## Composition

A rule declares its `subrules` up front; the evaluator resolves them **depth-first in
Rust before the script runs** ([`evaluate/compose.rs`](../../crates/rubix-rules/src/evaluate/compose.rs)),
collecting each child's `value` into the map `invoke()` reads. So `invoke("comfort-risk")`
inside a script returns that child's decision value. Because composition is resolved
ahead of script execution, evaluation stays deterministic and I/O-free, and sub-rule
spans nest under the parent's correlation id.

## Two execution paths

1. **Production evaluation** ([`evaluate/mod.rs`](../../crates/rubix-rules/src/evaluate/mod.rs)):
   mint a correlation id → evaluate root + sub-rules depth-first (emitting and persisting
   spans) → **record the decision as an insight through the gate** on
   `Capability::RuleInvoke` ([`evaluate/record.rs`](../../crates/rubix-rules/src/evaluate/record.rs))
   → publish the firing on the bus. The insight content carries `fired/value/reason/scores/group_id`
   and the correlation id. A gate denial or write failure aborts the whole evaluation
   (no partial insights).
2. **Dry-run** ([`evaluate/dryrun.rs`](../../crates/rubix-rules/src/evaluate/dryrun.rs)):
   the same `run_script()`, run against real history, but **side-effect-free** — no
   insight, no event, no persisted spans. It returns the decision plus the resolved input
   buckets, so the studio shows exactly what a save-and-fire would do.

Note the **capability split**: `RuleDefine` gates authoring/editing a rule;
`RuleInvoke` gates recording a firing. They are deliberately separate grants.

## How rules fire

Rules run on the after-write **hook** path ([HOOKS-AND-FILES.md](HOOKS-AND-FILES.md)): a
`kind:"hook"` record binds "on write of kind X, fire rule Y". The dispatcher fires the
rule after commit through a per-namespace system principal, and a recursion guard skips
writes whose `kind` matches any rule's `output`, so hook→rule→insight loops are
structurally impossible. Dry-run is the only other entry point.

## HTTP surface (Rules Studio)

[`http/rules/`](../../crates/rubix-server/src/http/rules):

| Method & path | Capability | Purpose |
| --- | --- | --- |
| `POST /rules` | `RuleDefine` | create (validates slug, script compiles, bindings; `409` on name clash) |
| `GET /rules` | — (scoped read) | list visible rules |
| `GET /rules/:name` | — | read one (`404` if not visible) |
| `PATCH /rules/:name` | `RuleDefine` | replace definition (name immutable) |
| `DELETE /rules/:name` | `RuleDefine` | delete (`204`) |
| `POST /rules/:name/dryrun` | — | side-effect-free run of a draft against history |
| `GET /rules/catalog?table=…` | — | discover bindable fields + filter facets for a table |
| `GET /rules/:name/referencing` | — | which rules compose this one (blast radius before edit/delete) |

The seed ([`seed/rules.rs`](../../crates/rubix-server/src/seed/rules.rs)) plants a
progression — thresholds → multi-input bands → composition → scoring → nested
composition — plus the hooks that fire them.

## Relationship to traces & evaluations

A rule evaluation is a `rule`-kind span ([§5a reserved keys](LAMINAR-BORROW.md), status
`Ok`, tokens/cost unset — rule eval has neither). Every span folds into the
[`trace_summary`](LAMINAR-BORROW.md) rollup before sampling, so the per-correlation-id
summary stays accurate. The decision's `scores` + `group_id` make a firing a **comparable,
chartable evaluation datapoint** (§5c) — the same shape that later covers agent-run QA
once the Rig brain lands ([AGENT.md](AGENT.md)).

## Open / deferred

- **Evaluation charting (§5c).** The datapoint shape (`scores`/`group_id`) is written
  today; dashboards that trend evaluation groups over time are LAMINAR-BORROW follow-on.
- **Agent-run evaluations.** Same recording path, blocked on the agent brain
  ([AGENT.md](AGENT.md)).
- **Per-rule schedule / streaming triggers.** Today rules fire on the after-write hook
  path or via dry-run; a time-scheduled or streaming-threshold trigger is not modeled.

## Authority

- [SCOPE.md](../SCOPE.md) — rules fire offline; two-path authorization; two authz layers.
- [HOOKS-AND-FILES.md](HOOKS-AND-FILES.md) — the after-write dispatcher that fires rules.
- [LAMINAR-BORROW.md](LAMINAR-BORROW.md) §5 — span typing, trace rollup, evaluation
  datapoints the decision feeds.
- [ADMIN-API.md](ADMIN-API.md) — `RuleDefine`/`RuleInvoke` grant management.
