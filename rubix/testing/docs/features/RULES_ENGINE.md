# Feature — Rules Engine (composable spark rules)

> Verified: **verified (live)** on `rubix-gaps` (2026-06-13). The `rubix-rules`
> crate is built and its own suite is green (`cargo test -p rubix-rules`, 71 tests,
> clippy clean, `unsafe_code = "forbid"`), **and** it is now wired into the running
> stack: a board `rule` node (`rubix-flow`), an org-scoped stored-rule store +
> `/rules` routes (`rubix-server`), and the severity map at the emit boundary. The
> live gates L1–L3 below are green. Source: `crates/rubix-rules/`,
> `crates/rubix-flow/src/node/rule/`, `crates/rubix-server/src/{store,api,flow}`.

Covers: the sandboxed Rhai rule layer that turns queried rows into a finding —
the "compute node" the board graph never had. A rule is a script that *orchestrates*
(thresholds, naming, composition) over a DataFusion-backed vectorized engine that
*computes* (the curated primitives). Design source of truth:
[docs/design/rules-engine.md](../../docs/design/rules-engine.md).

**Scope of what exists today:** the standalone engine — the curated primitive
surface, the rule-result + `finding` constructor, the `RuleStore` trait +
in-memory store, and the `rule(name, frame, params)` composition primitive with
cycle/depth bounding (`crates/rubix-rules`, no other rubix dep) — **plus** the
three integration seams now built: the board `rule` node
(`crates/rubix-flow/src/node/rule/`), the org-scoped stored-rule store + `/rules`
routes (`crates/rubix-server`), and the `Severity` → `SparkSeverity` map at the
emit boundary.

Prereq (library gates): `cargo test -p rubix-rules` runs with no stack, no zenoh,
no DB. The live gates (L1–L3) run against the integrated board/HTTP path.

---

## What to prove

1. **The non-negotiable design rule holds:** the script orchestrates, the engine
   computes — no script can iterate rows, and no primitive can grow a frame.
2. Each curated primitive computes the right values (golden frames).
3. The sandbox kill-switches stop a pathological script (ops, size, deadline,
   import).
4. Composition works and is bounded: a rule calls another; a cycle / over-depth /
   missing name is a clean `resolve` error, never a hang or panic.
5. The four error categories are distinct and nothing panics across the Rhai edge.

---

## Runbook (library gates — runnable today)

All gates are the crate's own test suite. CWD `rubix/`.

### 1. The whole suite

```bash
cargo test -p rubix-rules
cargo clippy -p rubix-rules --all-targets
```

✅ 71 tests pass; clippy clean. `unsafe_code = "forbid"` is inherited from the
workspace lints.

### 2. No-row-explosion invariant (the hard constraint)

```bash
cargo test -p rubix-rules --test frame_compute
```

✅ Every row-preserving primitive (`zscore`, `rolling_*`, `lag`, `anomalies`,
`sort`) returns exactly its input row count; every shrinking primitive
(`filter_*`, `head`/`tail`, `resample`) only shrinks; a chained pipeline stays
bounded. The surface offers **no join**, and the single compute path asserts
`out_rows <= in_rows`, so an explosion inside the engine is structurally
impossible — the thing the sandbox size limits cannot catch.

### 3. Every primitive against golden frames

```bash
cargo test -p rubix-rules --test 'frame_*'
```

✅ `select`/`rename`, `filter_gt/lt/eq`, `rolling_mean/min/max/sum` (time-duration
`RANGE` windows), `zscore`, `anomalies`, `resample` (`date_bin` + group-by),
`lag`/`diff`/`pct_change`, `fill_null` (zero/mean), `head`/`tail`/`sort`,
`describe`, `any_true` — each checked against a known input/output frame.

### 4. Sandbox kill-switches

```bash
cargo test -p rubix-rules --test sandbox_build
```

✅ An infinite loop trips `max_operations`; an oversized string and an oversized
array trip the size caps; a slow loop under a tiny `timeout` (but a huge op
budget) is stopped by the **wall-clock deadline**; `import "anything"` is blocked
by the `DummyModuleResolver` — the documented Rhai footgun (absence of
registration is *not* enough to block `import`). All surface as
`RuleError::LimitExceeded` (or a fail-closed error for `import`), never a hang.

### 5. Composition: success, cycle, over-depth, missing name

```bash
cargo test -p rubix-rules --test compose_guard
cargo test -p rubix-rules --test compose_budget
```

✅ A rule calls another stored rule and reads its verdict; a **direct** cycle
(`rule("self")`) and a **transitive** cycle (`a→b→a`) each return a clean
`RuleError::Resolve` (message contains `cycle`); a chain past the depth cap
(`DEFAULT_MAX_DEPTH = 8`) returns a `Resolve` error (message contains `depth`); a
missing composed name fails closed (`Resolve`); a missing required param is a
clear `Runtime` error. The op budget and deadline are **one allowance for the
whole tree** (a diamond `d→{b,c}→shared` resolves once via per-tick memoization;
an exhausted budget fails closed instead of granting a fresh allowance).

### 6. Error surfaces — four distinct categories, no panics

```bash
cargo test -p rubix-rules --test error_surface
```

✅ Malformed script → `Compile`; bad primitive argument / thrown error /
non-result return / unknown severity string → `Runtime`; the limit cases above →
`LimitExceeded`; composition failures → `Resolve`. Each crosses the Rhai edge as a
typed `RuleError`, never a panic. An empty / non-flagged result is **not** an
error — it is the normal "ran, found nothing" outcome (`run_run_rule.rs`).

---

## Runbook (live gates)

These run against the integrated stack. The board path is covered by
`crates/rubix-flow/tests/board.rs` (in-process) and the HTTP path by
`crates/rubix-server/tests/api_tests/rules.rs` (`POST /api/v1/boards/run` over a
seeded point). CWD `rubix/`.

```bash
cargo test -p rubix-flow --test board rule_node
cargo test -p rubix-server --test api rules
```

### L1. Board rule node — query → rule → emit_spark ✅

A board wires `query_his` → `rule` → `emit_spark`. The `rule` node builds a
`Frame` from the query's history rows, calls `run_rule` with the node's params,
and emits a structured finding on its `finding` outport into `emit_spark`'s
`finding` inport; a non-flagged result is a `clear` no-emit, a `RuleError` or an
input caps breach fails the node.

✅ `rule_node_flags_and_emits_with_rule_severity` (in-process) and
`inline_rule_board_emits_spark_with_rule_severity` (HTTP) run the full path and
assert one spark lands. `rule_node_clear_result_emits_no_spark` and the
caps-breach / broken-script tests prove the fail and no-emit branches.

### L2. Stored rule, resolved by name from a real store ✅

A rule saved in the org-scoped `rules` table is loaded by name for a board node
referencing it and for `rule(name, …)` composition. Resolution is fail-closed: a
missing name errors the node and emits nothing.

✅ `stored_rule_board_resolves_by_name` (HTTP, via `POST /orgs/{org}/rules` then a
board referencing it) and `rule_node_resolves_a_stored_rule_by_name` (in-process);
`missing_stored_rule_fails_closed_no_spark` proves fail-closed. The
referencing-rules listing is checked by `referencing_lists_the_change_impact`.

### L3. Severity maps to the canonical rubix severity / finding path ✅

A flagged `RuleResult` becomes a real spark at the rule's own severity — the
`rubix_rules::Severity` → `rubix_core::SparkSeverity` map (`rubix-flow`
`node::rule::map_severity`) is applied at the emit boundary, so a rule's
`finding("fault", …)` records a **fault** even when the `emit_spark` node's static
`severity` config says `info`.

✅ The L1 tests assert the spark severity equals the rule's verdict severity
(`fault`/`warning`), not the node config's `info`.

---

## Acceptance criteria ("done")

Library (today):

- [x] No script can iterate rows; no primitive grows a frame (`frame_compute`).
- [x] Every curated primitive verified against golden frames (`frame_*`).
- [x] Sandbox stops infinite loop / oversized string / oversized array / slow
      script / `import` (`sandbox_build`).
- [x] Composition succeeds; cycle / over-depth / missing name are clean `Resolve`
      errors, never a hang or panic (`compose_guard`).
- [x] Budget + deadline are one allowance for the whole tree; per-tick memoization
      (`compose_budget`).
- [x] Four error categories distinct; no panic across the Rhai edge
      (`error_surface`).
- [x] `cargo clippy -p rubix-rules` clean; `unsafe_code = "forbid"`.

Live (integrated):

- [x] L1 — board `rule` node runs query→rule→emit_spark end to end.
- [x] L2 — a stored rule loads by name from a real org-scoped store
      (fail-closed on a missing name); referencing-rules listing works.
- [x] L3 — a flagged result emits a real spark with the canonical severity.

---

## Integration seams (built)

1. **Board `rule` node** in `rubix-flow` (`src/node/rule/`) — between `query_his`
   and `emit_spark`. Builds a `Frame` from the query rows (`frame.rs`), calls
   `rubix_rules::run_rule(store, source, frame, params, limits)`, and emits a
   structured `{message, severity}` finding into `emit_spark`'s new `finding`
   inport. The sync engine is bridged into the async actor via `block_in_place`.
2. **Org-scoped `RuleStore`** — `TableRuleStore` (`rubix-server`
   `src/flow/rule_store.rs`) backs `rubix_rules::RuleStore::load` with the `rules`
   table (`src/store/rules.rs`, sqlite + postgres), with CRUD + the referencing
   listing exposed at `/api/v1/orgs/{org}/rules`. Resolution is fail-closed. The
   board's tenant org is derived from its keyexpr configs (`BoardGraph::tenant_org`).
3. **Severity / finding mapping** — `rubix-flow` `node::rule::map_severity` maps
   `rubix_rules::Severity` onto `rubix_core::SparkSeverity` at the emit boundary;
   `rubix-rules` stays standalone (no `rubix-core` dep).

Deferred by design (not built, per the scope): stored **functions** + the
`call(name, frame, params)` primitive (this crate ships the rule/verdict half;
the `RuleStore` trait is shaped to add a function store alongside without changing
the rule path), pinned/versioned composition, and the UI editor.

---

## Gotchas

- **DataFusion-first, not Polars** — by design (no second Arrow stack). Most
  primitives are DataFusion window/aggregate SQL over a per-call `SessionContext`;
  `resample` is `date_bin` + group-by. The primitive surface is backend-agnostic,
  so the engine choice does not leak into scripts.
- **`rolling_*` / `lag` take an explicit `time_col`** as the first argument. The
  design's signature omits it, but a `RANGE`/window frame is undefined without
  naming its ordering column; guessing one would be fragile. Resolved in-code with
  a comment (`frame/rolling.rs`, `frame/lag.rs`).
- **`describe` / `any_true` bypass the per-row no-growth guard** via a separate
  reducing compute path: they emit a fixed small row count (which can be 1 row from
  0 input rows), which the strict `out_rows <= in_rows` guard would misread as
  growth. They cannot explode (a guard-free aggregate without a large group-by key
  yields a handful of rows). Documented in `frame/compute.rs`.
- **Errors are potentially data-bearing.** A Rhai runtime error can interpolate
  script strings and queried row values into its message; the surfaced `RuleError`
  string payloads should be treated as tenant data when a caller decides what to
  log vs show.

## Known issues / fixes

Library-verified 2026-06-13 on `rubix-gaps`, then **integrated and verified live**
the same day. The crate was built to
[docs/design/rules-engine.md](../../docs/design/rules-engine.md); its own suite is
green and the L1–L3 board/HTTP path is green. No backend bug found in the engine
during integration. Three engine-design ambiguities were resolved in-code earlier:

1. **`rolling_*`/`lag` signature** — added an explicit `time_col` (a window's
   ordering column cannot be guessed). See Gotchas.
2. **Script→decision bridge without row iteration** — added `any_true(col)`, a
   `bool_or` engine reduction, so a flag column (e.g. `anomalies`) becomes a script
   bool without looping rows.
3. **Reducing primitives vs the no-growth guard** — `describe`/`any_true` route
   through a separate reducing compute path. See Gotchas.

Integration resolved four further design ambiguities, each kept the engine
standalone (no new rubix-crate dep into `rubix-rules`):

4. **Severity authority vs "`emit_spark` needs no change".** The design wires the
   `RuleResult` to `emit_spark`'s value inport but `emit_spark` read severity only
   from static config — which would silently override a rule's `finding("fault")`.
   Resolved by giving `emit_spark` an additive optional `finding` inport carrying
   `{message, severity}`; when connected the rule's verdict is authoritative, and
   the legacy scalar `value` path is unchanged. The rule node emits onto it; the
   `Severity → SparkSeverity` map lives in `rubix-flow`.
5. **Rule tenancy.** A tenant is `{org}/{site}` but boards carry no org binding.
   Rules are **org-scoped** (name unique per org); the board's org is derived from
   its keyexpr node configs (`BoardGraph::tenant_org`), fail-closed when none.
6. **Sync engine in an async actor.** `run_rule` drives DataFusion on a per-call
   `block_on`, which panics inside a Tokio worker. The node bridges via
   `tokio::task::block_in_place` (direct call off a multi-thread runtime).
7. **API gaps for integrators.** `run_rule` takes a `rhai::Map` and the crate only
   re-exported `SchemaRef`/`RecordBatch` — too little to build the inputs without a
   direct `rhai`/`arrow` dep. Added `params_from_json`/`Params` and re-exported the
   `arrow` module; the crate stays standalone.

Touched outside the crate: the board `rule` node + `emit_spark` `finding` inport
(`rubix-flow`), the `rules` table + store + `/rules` routes + `rule_store()` wiring
(`rubix-server`).
