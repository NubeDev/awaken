# Feature — Rules Engine (composable spark rules)

> Verified: **library-verified** on `rubix-gaps` (2026-06-13). The `rubix-rules`
> crate is built and its own suite is green (`cargo test -p rubix-rules`, 71 tests,
> clippy clean, `unsafe_code = "forbid"`). It is **not yet wired into the server**,
> so the live HTTP/board gates below are **blocked on integration**, not verified —
> a separate session owns wiring. Source: `crates/rubix-rules/`.

Covers: the sandboxed Rhai rule layer that turns queried rows into a finding —
the "compute node" the board graph never had. A rule is a script that *orchestrates*
(thresholds, naming, composition) over a DataFusion-backed vectorized engine that
*computes* (the curated primitives). Design source of truth:
[docs/design/rules-engine.md](../../docs/design/rules-engine.md).

**Scope of what exists today:** the engine, the curated primitive surface, the
rule-result + `finding` constructor, the `RuleStore` trait + in-memory store, and
the `rule(name, frame, params)` composition primitive with cycle/depth bounding.
It is a **standalone library**: it depends on no other rubix crate, and queries no
database — the caller hands in the rows. There is **no board node, no HTTP route,
no stored-rules table** yet; those are the named integration seams below.

Prereq (library gates): `cargo test -p rubix-rules` runs with no stack, no zenoh,
no DB. The live gates need the integration that does not exist yet.

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

## Runbook (live gates — BLOCKED on integration, do not check yet)

These cannot run today: there is no rule board node and no HTTP route. They are
written now so the integrating session knows exactly what "done" looks like.

### L1. Board rule node — query → rule → emit_spark ⛔

A board wires a `query_his` node into a new `rule` node into `emit_spark`. The
rule node calls `run_rule` with the query's `RecordBatch`es as the input frame and
the node's params; the `RuleResult` drives `emit_spark`.

⛔ **Blocked:** no `rule` board component exists in `rubix-flow` yet. → seam (1).

### L2. Stored rule, resolved by name from a real store ⛔

A rule saved in a tenant-scoped rules table is loaded by name for composition and
for a board node referencing it by id.

⛔ **Blocked:** the only `RuleStore` today is `MemoryRuleStore` (tests/fixtures).
No migration, no table, no CRUD route. → seam (2).

### L3. Severity maps to the canonical rubix severity / finding path ⛔

A flagged `RuleResult` becomes a real spark finding with the correct rubix
severity.

⛔ **Blocked:** the crate uses a local `Severity` mirror (`info`/`warning`/`fault`)
on purpose (standalone). The map onto `rubix-core` severity + the finding path is
the integrator's. → seam (3).

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

Live (blocked on integration — see seams):

- [ ] L1 — board `rule` node runs query→rule→emit_spark end to end.
- [ ] L2 — a stored rule loads by name from a real tenant-scoped store.
- [ ] L3 — a flagged result emits a real spark with the canonical severity.

---

## Integration seams (named, untouched — for the follow-up session)

1. **Board `rule` node** in `rubix-flow` — between a query node and `emit_spark`.
   Calls `rubix_rules::run_rule(store, source, frame, params, limits)` with the
   query's batches as the `Frame`; hands the `RuleResult` to `emit_spark`.
2. **Real `RuleStore`** — a tenant-scoped table-backed implementation of the
   `rubix_rules::RuleStore` trait (`fn load(&self, name) -> Result<StoredRule>`),
   plus the design's referencing-rules listing. Swapped in for `MemoryRuleStore`.
3. **Severity / finding mapping** — map `rubix_rules::Severity`
   (`info`/`warning`/`fault`) onto the canonical `rubix-core` severity and the
   existing finding path at the emit boundary.

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

Library-verified 2026-06-13 on `rubix-gaps`. The crate was built to
[docs/design/rules-engine.md](../../docs/design/rules-engine.md) and its own suite
is green; no backend bug found (there is no integrated backend path yet to find one
in). Three design-doc ambiguities were resolved in-code, each noted with a comment:

1. **`rolling_*`/`lag` signature** — added an explicit `time_col` (a window's
   ordering column cannot be guessed). See Gotchas.
2. **Script→decision bridge without row iteration** — added `any_true(col)`, a
   `bool_or` engine reduction, so a flag column (e.g. `anomalies`) becomes a script
   bool without looping rows.
3. **Reducing primitives vs the no-growth guard** — `describe`/`any_true` route
   through a separate reducing compute path. See Gotchas.

No other rubix crate or file was touched except the one workspace `members` line
needed to compile the crate.
