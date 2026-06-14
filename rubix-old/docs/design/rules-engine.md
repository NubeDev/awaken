# Rules Engine — Composable Spark Rules (Rhai over a vectorized engine)

Scope for authoring spark rules: a sandboxed scripting layer that turns queried
data into findings, where the script orchestrates and a vectorized engine
computes, and where a rule can call other rules. This replaces the gap where a
spark board today can query history and emit a finding but has nowhere to express
the actual decision logic ("flag when avg temp > 25"), and no way to reuse logic
across rules.

## Problem

A spark today is a `BoardGraph` — a JSON graph of wired nodes (`trigger`,
`query_his`, `emit_spark`, …). That model can move data and fire a finding, but:

1. **No compute.** There is no node that evaluates rule logic. A threshold check,
   an anomaly score, or "is this a finding?" has nowhere to live except an LLM
   (`agent_call` — non-deterministic, costly per tick) or a future hardcoded
   actor.
2. **No composition.** A board is a standalone document. One rule cannot call
   another. Shared logic ("rollup temp and check threshold") is copied into every
   board that needs it — the duplication the project rules forbid, with no
   mechanism to avoid it.

The authoring model people reach for is the SkySpark/Axon one: write a rule, then
build bigger rules out of rules you already wrote. The current graph model cannot
express that.

## Approach

Add a sandboxed **Rhai** scripting layer for rule logic, structured so the script
is the glue and a vectorized engine does the data work. This is a port of the
nexus `RW-06` insights-engine design
(`nexus/docs/scope/nextgen/rewrite/RW-06_INSIGHTS_ENGINE_RHAI.md`), adapted from
"transform a DataFrame for a dashboard" to "decide a finding for a spark", plus a
composition primitive RW-06 does not have.

### Design rule (non-negotiable)

> **The script orchestrates; the engine computes.** User code never loops over
> rows. It composes vetted vectorized primitives and writes decision logic. This
> is what makes the language both fast and sandboxable.

A rule script writes `if`/thresholds/naming and calls curated primitives
(`rolling_mean`, `resample`, `zscore`, `anomalies`); it does not iterate rows in
Rhai. The data work runs in the engine, not in the interpreter.

### Why Rhai over the alternatives

- **vs. full Axon-style language** — Axon evaluates over grids in the language
  itself: powerful but slow and hard to sandbox, and a large build. Rhai-as-glue
  with engine-side compute is faster and safer for the same expressiveness.
- **vs. a custom DSL** — more work, less safety, no ecosystem. Rhai ships the
  sandbox controls (operation/recursion/size limits, deadlines) this needs.
- **vs. SQL-only logic** — SQL carries the heavy data work (and should — see data
  path below), but it cannot express composition or multi-step decision logic.
  The rule layer sits above the query.

## Data path

Logic flows **query DB → rule language → finding**, matching the spark wiring:

```
  trigger ─► query ──────────────► rule (Rhai) ──────► emit_spark
            └ his / datasource /   └ thresholds,       └ finding on
              SQL — heavy data       composition,         spark/**
              work in the engine     finding decision ┘
```

- **Query** — an existing `query_his` node, or the external datasource
  (see [datasources.md](datasources.md)), returns rows. SQL does as much of the
  heavy lifting as it can (rollups, windows, filters pushed to the database).
- **Rule** — a new board node runs a stored or inline Rhai script over those rows.
  It runs the vectorized primitives the query could not, applies the decision
  logic, and may call other **functions** and **rules** (see
  [Composition](#composition--functions-and-rules)). It produces a typed rule
  result (`flagged`, `severity`, `message`, optional `value`), not a reshaped
  frame.
- **emit_spark** — fires the finding from the rule result, unchanged.

Composed callees do **not** run their own queries. All database reads happen at
the board edge (the query node), where caps and truncation are enforced; a rule
and everything it composes operate on the caller-supplied frame. This keeps the
diagram true, keeps a shared sub-rule from fanning out N re-reads of the external
historian on every tick, and keeps every read under one cap boundary.

The missing "compute node" from the graph model is this rule node.

## Composition — functions and rules

This is the SkySpark-style reuse the graph model could not express, and the main
addition over RW-06.

### Functions are the reuse unit; rules are the emit unit

The reusable half of "rollup temp and check threshold" is the rollup — it
produces a frame or a score, not a verdict. If the only stored unit returned a
verdict, every rule would re-derive its intermediate values and the duplication
would simply move one level down. So there are two stored kinds, mirroring Axon
(the reusable thing is a *function* that returns values, not a rule):

- **Function** — a stored, named, parameterized Rhai script that returns a
  **value or a frame handle**. Reusable building block. **Not** wireable to
  `emit_spark`. This is what shared logic ("rollup + zscore") lives in.
- **Rule** — a stored, named, parameterized script that returns a **verdict**
  (`flagged`, `severity`, `message`, optional `value`). Only a rule can drive
  `emit_spark`.

Both are tenant-scoped, mirroring RW-06's stored `insights` table, with a declared
**parameter schema** and a declared **input contract** (the columns the script
expects on the caller-supplied frame) so a composition mismatch fails clearly
rather than opaquely at runtime.

### Calling

Two curated primitives are registered into the engine:

- `call(name, frame, params)` — run a stored **function** over the caller's frame,
  return its value/frame.
- `rule(name, frame, params)` — run a stored **rule** over the caller's frame,
  return its verdict.

The frame is passed in explicitly (callees never query). A rule composes
functions for the shared compute and other rules for combined verdicts:

```
let temps = call("rollup-temp", df, #{ every: "1h" });   // shared compute → frame
let hi    = rule("temp-high", temps, #{ limit: 25.0 });
let co2   = rule("co2-stale", df,    #{ point: ctx.point });
if hi.flagged || co2.flagged {
    finding("fault", `AHU unhealthy: ${hi.msg}; ${co2.msg}`)
}
```

### Bounds — one budget for the whole tree

`set_max_call_levels` alone is not enough: a fresh per-callee operation/time
budget would make the total allowance `op-limit × depth × fan-out`, so composition
would multiply the very limits the sandbox sets. Instead the **operation budget
and wall-clock deadline are a single allowance for the whole composition tree**,
decremented across every `call`/`rule` invocation. One engine, one budget, shared.

- **Per-tick memoization.** `call`/`rule` results are memoized within a single
  tick keyed by `(name, frame-identity, params)`, so a popular shared function
  invoked by several rules runs once per tick, not once per caller.
- **Cycle guard ownership.** The cycle and composition-depth guard is enforced by
  the **executor at call time**, not the store at save time — only call-time
  enforcement catches a dynamically-named `rule(x)` where `x` is computed. A
  cycle or over-depth is a distinct `resolve` error, never a hang or panic.

This is the unit of reuse: write a function once, then build rules — and bigger
rules — on top of it.

## Versioning of composed names

Composition resolves stored functions/rules **live by name** in v1. Editing a
shared function therefore changes every rule built on it, effective on the next
tick. This is the known name-addressed-composition hazard, and it is in tension
with the platform's pinning/fail-closed stance (ADR-0035). The v1 position:

- Resolution is **fail-closed** — a missing or mistyped composed name is a
  `resolve` error that fails the rule, never a silent skip.
- The store exposes a **referencing-rules listing** — given a function/rule name,
  which rules compose it — so an operator can see the change-impact before editing
  a shared unit.
- **Pinned/versioned composition is explicitly deferred** to a follow-up that
  brings stored rules under the ADR-0035 model. v1 does not version; it makes the
  blast radius *visible* rather than invisible.

## Sandbox

Lifted from RW-06's `sandbox.rs`, non-negotiable for running authored scripts on
a scheduled cadence:

- `set_max_operations`, `set_max_call_levels`, `set_max_string_size` /
  `array_size` — bound runaway scripts.
- Wall-clock timeout via `on_progress` + deadline.
- **No file / network / eval APIs registered.** Imports disabled **explicitly**
  via a dummy module resolver — absence of registration is not enough to block
  `import`.
- One engine per execution (cheap), no cross-tenant state.

## Vectorized engine

The compute backend behind the curated primitives.

- **DataFusion-first**, not Polars. Polars ships its own Arrow fork, which means a
  copy/FFI bridge and a second Arrow stack in the binary — the dependency bloat
  this platform avoids. Most primitives are plain DataFusion window/aggregate
  expressions over the session context; `resample` is `date_bin` + group-by (what
  a Timescale user expects anyway). The primitive surface is backend-agnostic, so
  the engine choice does not leak into scripts.
- **Curated primitive surface** — `select`/`rename`/`filter_*`, `rolling_mean`/
  `min`/`max`/`sum(col, window)`, `zscore(col)`, `resample(time_col, every,
  aggs)`, `lag`/`diff`/`pct_change(col)`, `fill_null(strategy)`, `head`/`tail`/
  `sort`, `anomalies(col, z_threshold)`, `describe()`. Scripts chain them on
  DataFrame handles.
- **`rolling_*` windows are time-duration, not row-count.** Sensor data is
  irregular, so a row-count window is almost never what a building-analytics
  author means. `window` is a duration and the implementation uses DataFusion
  `RANGE` frames over the time column. This constrains the primitive
  implementation, so it is pinned here.
- **No primitive may increase row count beyond its input** (no joins / cross
  products). The sandbox's size limits cannot catch an explosion that happens
  inside the engine, so the curated surface must make explosion impossible. This
  matters more for sparks than for dashboards: a scheduled rule that explodes rows
  runs on every tick.

## Rule result and findings

The divergence from RW-06: an insight returns a reshaped DataFrame for a
dashboard; a spark rule returns a **decision**.

- A `finding(severity, message)` constructor and a typed rule result
  (`flagged: bool`, `severity`, `message`, optional `value`) are the rule node's
  return type. `severity` is a string (`"info"`/`"warning"`/`"fault"`). The
  optional `value` carries a score/number a composing rule can read without
  re-deriving it — the same reuse motivation that splits functions from rules.
- `emit_spark` consumes the rule result and persists/publishes the finding through
  the existing finding path — unchanged.

## Truncation and caps on the spark path

Caps still apply, and breach semantics follow [datasources.md](datasources.md):

- A rule folding **truncated** input rows into a finding can silently reach a
  wrong conclusion. So a caps breach on the input to a spark rule is an **error**
  that fails the node, not a finding emitted from partial data.
- Output caps still apply after the rule (it can aggregate down, never explode
  past caps), consistent with the "no primitive increases row count" rule.

## Errors

One `thiserror` error domain for the rule engine, distinguishing:

- **compile** — the script is malformed,
- **runtime** — the script ran and failed,
- **limit-exceeded** — a sandbox limit (operations, recursion, size, deadline)
  tripped,
- **resolve** — a composed `rule(name)` does not exist, or a cycle/over-depth was
  detected.

Sparks must distinguish "the rule is broken" from "the rule ran and found
nothing" — a broken rule is an operational error, an empty result is a normal
non-finding. Errors are structured and safe to surface to a tenant — noting that
a Rhai runtime error can interpolate script strings and row values into its
message, so the surfaced error categories must be treated as potentially
data-bearing rather than passing raw engine text through unconsidered.

## Dry run

Rules execute on a schedule against live data, so an author needs to validate one
before wiring it into a board. The engine exposes a **dry-run** entry point: run a
rule (inline or stored) against a caller-supplied frame (e.g. a chosen time range)
and return the rule result **without emitting** a finding. This is in scope here
(it is the same executor path with the emit step suppressed) and is what the
eventual UI editor will call.

## Relationship to existing components

- **`rubix-flow`** — gains one new board component, the `rule` node, sitting
  between a query node and `emit_spark`. The actor model, board runner, schema,
  and finding emitter are unchanged. `emit_spark` already accepts a computed
  message on its `value` inport, so the rule result wires in with no change to it.
- **`rubix-query` / datasources** — the rule layer reads their output (rows); it
  does not change either. Heavy data work stays in SQL where it belongs; the rule
  layer is the post-query decision/composition stage.
- **`agent_call`** — remains for genuinely fuzzy judgments. The rule layer is the
  deterministic answer for thresholds, scoring, and composition; the LLM is not
  the general mechanism for rule logic.

## Non-goals

- No per-row Rhai callbacks into the engine (performance and sandbox hazard).
- No Polars (second Arrow stack), no Python.
- No primitive that joins or otherwise increases row count.
- No file / network / eval / import access from scripts.
- No rule UI editor in this scope (a follow-up surface). This scope is the engine,
  the stored-rule store, the composition primitive, and the board node.
- No change to the spark engine, board runner, or finding path beyond adding the
  one rule node.
