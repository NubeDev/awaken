# Testing TODO — Start Here, In This Order

> The work queue for a session picking up this suite. Do it **top to bottom** —
> the order is dependency order: each phase relies on the one above it being
> green. Read [README.md](README.md) first for the operating contract, then start
> at Phase 0.

The job of this suite right now is to **flip every runbook from `scaffold` to
`verified`** — run it live, confirm each ✅ gate, fix the doc or the backend where
reality differs, bump the doc's `Verified:` line. Once a runbook is verified, its
gates become regression checks. Only after the foundation is verified do you go
hunting for backend bugs.

**The loop for every item below:** run the runbook's steps → on a ✅ failure go to
[feedback-loop/](feedback-loop/) (capture → triage → fix) → re-run → record the
result in that doc's "Known issues / fixes" → bump `Verified:`.

---

## Phase 0 — Can the stack even come up? (do this first, once) ✅ DONE 2026-06-13

- [x] **Build + boot.** `make build-be` → `drivers.json` → `make dev-be`. ✅
      `/healthz` returns `{"status":"ok","version":"0.1.0"}` on `:8088`; sim driver
      spawns and attaches. → [00_setup/QUICKSTART.md](00_setup/QUICKSTART.md).
- [x] **Live data path end to end.** Provision `nube/hq/ahu-3/temp`; sim's
      `cur_value` lands (oscillating 19–23) and `his` grows; `/query` count ==
      direct sqlite count. ✅ QUICKSTART steps 4–6 green.
      - **Bug found + fixed:** the inbound `**/cur` subscriber was missing — driver
        publishes never reached the store. Added `bus/subscribe_cur.rs`
        (`allowed_origin(Remote)` to avoid a self-ingest loop) + a regression test.
        See [features/ZENOH_BUS.md](features/ZENOH_BUS.md) "Known issues / fixes".
      - **Teardown gotcha found:** `make kill` frees ports but doesn't reap a
        backgrounded `cargo run` server/driver, which then holds the SQLite WAL and
        leaks stale data into the next boot. QUICKSTART teardown updated with
        `pkill` guidance.
- [x] Stack is healthy → proceed to Phase 1.

Once Phase 0 is green you have a working rig. Everything below is verifying one
feature at a time on top of it.

---

## Phase 1 — Foundation (no zenoh/agent needed)

Verify the store + query layer with `RUBIX_ZENOH=0` if you want to isolate from the
bus. These have the most existing unit coverage, so failures here are most likely
doc/wiring bugs, not deep backend bugs — a gentle start.

- [x] **Points & priority array** — CRUD, lower-level-wins, relinquish, range
      check, history. ✅ verified live `d48dd8b2` 2026-06-13 (`RUBIX_ZENOH=0`).
      Backend correct on every gate; fixed 5 doc bugs in the runbook (required
      `display_name`, `?tags=a,b` filter param, `.point.*` response wrapping,
      sensor still serializes the array, `/cur`↔`/write` kind split). →
      [features/POINTS_PRIORITY_ARRAY.md](features/POINTS_PRIORITY_ARRAY.md)
- [x] **Query & rollup** — canonical tables, `points_cur`, read-only guard,
      bucketed aggregates, injection guard. ✅ verified live 2026-06-13. 5/6 gates
      green as-scaffolded; **found + fixed a backend gap**: `/query` had no read-only
      guard (a `DROP` returned `{"rows":[]}` — harmless against read-only providers,
      but not refused). Added `rubix_query::ensure_read_only` in the engine; both the
      HTTP route and agent tool now refuse writes with `400`. →
      [features/QUERY_AND_ROLLUP.md](features/QUERY_AND_ROLLUP.md)

## Phase 2 — Edge runtime (needs zenoh)

The driver/bus layer. `rubix-driver-sim/tests/supervised.rs` already proves the
live spawn lifecycle, so lean on it when a gate is ambiguous.

- [x] **Driver supervisor** — manifest load (missing/malformed/valid), spawn +
      attach, capability `covers`, access direction, out-of-grant fail-closed,
      shutdown reap. ✅ verified live `bdbd01f7` 2026-06-13. All 7 gates green;
      backend correct on every one, no doc or backend bug found (each gate maps to a
      passing unit/live test — see the doc's "Known issues / fixes"). →
      [features/DRIVER_SUPERVISOR.md](features/DRIVER_SUPERVISOR.md)
- [x] **Zenoh bus** — inbound/outbound `cur`, `**/write` + `**/his/**` queryables,
      owned-site silence, spark publish. ✅ verified 2026-06-13. Every gate proven by
      `rubix-server` `api_tests::bus` (7/7), which drives a real second zenoh peer
      (subscribe + `get`) against a live server — the "second peer" rig the runbook
      asks for. Backend correct on all gates incl. owned-site silence. →
      [features/ZENOH_BUS.md](features/ZENOH_BUS.md)

## Phase 3 — Orchestration

- [x] **Reflow boards** — inline run, read→write wiring, unknown-component
      fail-closed, stored/versioned boards, scheduler (interval + `trigger` node),
      `emit_spark`. ✅ verified live `bdbd01f7` 2026-06-13 (`RUBIX_ZENOH=1`). Backend
      correct on all 6 gates; fixed 4 doc bugs in the request bodies (`{"board":…}`
      run wrapper, `board` not `graph`, required `display_name`+tagged `trigger`,
      `kind:"sp"` not `"setpoint"`). → [features/BOARDS_REFLOW.md](features/BOARDS_REFLOW.md)

## Phase 4 — Agentic layer (needs `RUBIX_AI=1`)

The signature rubix capability and the most safety-critical. Verify the gating
bands carefully — a wrong band is a real safety bug, not a doc nit.

- [x] **AI tools & agent** — read, commit-band write, **escalation suspend (HITL)**,
      resume/cancel, deny band, spark dispatch, MCP, tool-boundary scope. ✅ verified
      live `bdbd01f7` 2026-06-13 against a real OpenAI run (`gpt-4o-mini`). All 8 gates
      green; every safety property exact (three-band boundaries, HITL one-shot, deny,
      scope fail-closed). Fixed 2 doc bugs (`thread_id` required; no `?origin=` filter).
      → [features/AI_TOOLS_AND_AGENT.md](features/AI_TOOLS_AND_AGENT.md)
      - **Build-blocker note (resolved 2026-06-13):** the earlier `rubix-datasource`
        `libsqlite3-sys` conflict is fixed — the crate now depends on `sqlx-core` +
        `sqlx-postgres` directly (no `sqlx` facade → no `sqlx-sqlite`), so only
        `libsqlite3-sys 0.35` (rusqlite's) resolves. The *current* workspace
        non-load is a **different** crate, `crates/rubix-rules/` (in-progress, no
        `src/lib.rs` yet) — owned by the rules session, not this suite.

---

## Phase 4b — Entity CRUD & tenancy loop (active)

The management surface every other feature provisions through, plus the org/site
tenant boundary. This runbook doubles as the **gap detector** for design
increment **A** ([../../docs/design/crud-and-tenancy.md](../../docs/design/crud-and-tenancy.md)):
the backend has **no `Update` verb on any entity** today, and widgets/sparks lack
Get/Delete. Gates marked **⟂ gap** are *expected red* on current `main` — they are
the work list, not regressions; they flip green as the endpoints land.

- [ ] **Entity CRUD & tenancy** — Part A: site/equip/point/board create-read-
      delete round-trips (live today) + the ⟂ `PATCH`/widget-spark Get-Delete gaps
      (red until increment A). Part B: tenant isolation (scoped-token site list
      excludes other orgs, cross-org write `403`, structural scoped-query
      confinement, `kfc/hq` ≠ `kfc/hq2`) — **needs auth ON**. First pass: confirm
      the non-⟂ gates green on `main`, leave the ⟂ gates documented-red, bump
      `Verified:` to **partial**. → [features/ENTITY_CRUD_AND_TENANCY.md](features/ENTITY_CRUD_AND_TENANCY.md)
  - [ ] **Implement increment A** (separate session, owns the backend change): add
        `PATCH /api/v1/{sites,equips,points,boards}/{id}` (metadata-only; identity
        fields immutable) + `update_*` store methods (concrete + Postgres), and
        Get/Delete for widgets + sparks. Then flip the ⟂ gates green → **verified**.

---

## Library crates (core built ahead of integration)

Standalone library crates that are fully unit-tested in isolation but not yet
wired into the running stack. Their gates are `cargo test -p <crate>`, not
live-stack runbooks — they flip to a live phase above once a session integrates
them. (See **library-verified** in [README.md](README.md).)

- [x] **External datasources** — read-only native-SQL passthrough to external
      databases (TimescaleDB/Postgres): manifest + registry (one pool per id,
      creds never logged), single-statement + bound-param executor, row/byte/
      wall-clock caps with a lenient/strict breach split, operator-registered
      named queries (the AI tier), and `information_schema`/declared-blob describe.
      ✅ library-verified 2026-06-13 — `cargo test -p rubix-datasource` 38 pass
      (+5 live-DB `#[ignore]`), clippy clean, `unsafe_code = forbid`. Core engine
      only; **no integration** (no HTTP route / spark node / AI tool /
      `datasources.json` loaded). → [features/DATASOURCE.md](features/DATASOURCE.md)
  - [ ] **Integration follow-up** (separate session): a `{datasource, sql}` widget
        binding + render path, a datasource spark node (breach = node error), and a
        read-only AI named-query tool under the existing gating. Then this flips
        from library-verified to a live feature gate.

- [x] **Rules engine** — sandboxed Rhai rule layer that turns queried rows into a
      finding: a DataFusion-first vectorized engine behind a curated primitive
      surface (`select`/`filter_*`/`rolling_*`/`zscore`/`resample`/`lag`/`diff`/
      `pct_change`/`fill_null`/`head`/`tail`/`sort`/`anomalies`/`describe`/
      `any_true`), the **no-row-explosion invariant** (no join; `out_rows <=
      in_rows` enforced), the sandbox factory (op/call/size caps, wall-clock
      deadline, `import` blocked by a dummy resolver), the `finding`/`RuleResult`
      return type, a `RuleStore` trait + in-memory store, and the `rule(name,
      frame, params)` composition primitive (one budget for the whole tree, cycle +
      depth-cap guard, per-tick memoization).
      ✅ library-verified 2026-06-13 — `cargo test -p rubix-rules` 71 pass, clippy
      clean, `unsafe_code = forbid`. Engine + composition only; **no integration**
      (no board `rule` node / HTTP route / stored-rules table / severity map). Built
      to [docs/design/rules-engine.md](../../docs/design/rules-engine.md). →
      [features/RULES_ENGINE.md](features/RULES_ENGINE.md)
  - [ ] **Integration follow-up** (separate session): the board `rule` node wiring
        query→rule→`emit_spark`, a real tenant-scoped `RuleStore` backed by a
        rules table (+ referencing-rules listing), and the `rubix_rules::Severity`
        → `rubix-core` severity / finding-path map. Then this flips from
        library-verified to a live feature gate (L1–L3 in the runbook).

---

## Phase 5 — Cross-feature scenarios (regression set)

Only meaningful once the feature phases above are green. These are the golden paths
that must *always* hold. → [scenarios/README.md](scenarios/README.md)

All four verified 2026-06-13 (`rubix-gaps`). S2/S3 live against a real OpenAI run;
S4 via the cross-feature `tenancy`/`scoped`/`bus` integration tests; S5 by two
clean-DB runs. No scenario-level backend bug. Per-gate evidence in
[scenarios/README.md](scenarios/README.md).

- [x] **S2** — rule board → spark → agent (the closed loop). ✅ stored board
      `read_point→emit_spark` → spark on the bus → dispatcher run `origin:dispatch`,
      completed. Whole Boards→Bus→Dispatch→Agent loop closed live.
- [x] **S3** — HITL escalation suspend/resume. ✅ prio-8 suspend (store untouched) →
      resume applies (`409` on re-resume) → cancel drops (`204`). Deny band per Phase 4.
- [x] **S4** — tenant isolation across `nube/hq` vs `nube/hq2`. ✅ tool-boundary
      refusal (dispatch + chat), scoped query, bus owned-site silence — all green.
- [x] **S5** — determinism: clean-DB re-run reproduces the same `his` sequence. ✅
      both runs reproduce the deterministic triangle **cycle** (no RNG). Nuance: the
      *start offset* is a provision-vs-tick race — assert the cycle/transitions, not
      `his[0]`. Recorded in the scenario doc.

## Phase 6 — Make it one command (do once the manual loops are stable)

- [x] Write `testing/scripts/capture.sh` from
      [feedback-loop/CAPTURE.md](feedback-loop/CAPTURE.md)'s one-shot block. ✅
      `capture.sh <scenario>` writes the standard bundle (env/point/query_count/
      db_state/openapi/symptom) and echoes its dir; `POINT=` adds `point.json`,
      `SYMPTOM=` seeds `symptom.md`. Path-independent (resolves `testing/` from
      `BASH_SOURCE`). Referenced from CAPTURE.md.
- [x] Write `testing/scripts/run-scenario.sh <name>` — stack up, run a scenario's
      gates, auto-capture on first ❌. ✅ `run-scenario.sh <S1..S5>` builds → cleans
      the store → boots `make dev-be` → waits on `/healthz` → runs that scenario's
      gates, calling `capture.sh` (+ `server.log` tail) on the first ❌ and tearing
      the stack down (with the orphan-reap `pkill`). S1/S5 drive the live data path
      (cur-in-band, his-grows, `points_cur`, triangle-cycle rotation check); S2/S3/S4
      run the cross-feature integration suites the scenarios doc cites as their proof
      (S2 falls back to the scripted `dispatch` test without `RUBIX_AI`+key).
      `KEEP_STACK=1` leaves the server up. → [feedback-loop/FIX_LOOP.md](feedback-loop/FIX_LOOP.md)
      "scripted driver". (`testing/scripts/` is git-ignored.)

---

## Now you're testing the backend, not the docs

With the suite verified, the feature docs' ✅ gates are live regression checks. From
here the work is **finding and fixing real rubix bugs**:

- Run the scenarios after any backend change; a newly-red gate is a regression.
- Pick at the **"Remaining" / "Gotchas"** items each feature doc lists (e.g. hot
  scheduler reconfiguration, the cross-mesh `points_cur` variant, partition
  predicate pushdown) — cross-check against `rubix/STATUS.md` "Remaining" lists so
  you don't re-file something already known.
- There are **no HTTP-level integration tests for the route handlers** yet
  (ARCHITECTURE §9) — promoting a verified runbook into a `crates/rubix-server/tests/`
  integration test is high-value follow-up.

Every fix follows [feedback-loop/FIX_LOOP.md](feedback-loop/FIX_LOOP.md)'s
"proving the fix" gate and gets recorded in the relevant feature doc.

---

## Conventions while working this list

- A doc carries `> Verified: <commit> on <date>`. When you verify it live, change
  `scaffold` → `verified` in both the doc header and the table in
  [README.md](README.md), and bump the date/commit.
- One feature per sitting. Don't half-verify three docs — fully green one, record
  it, move on. That's the same discipline as the `docs/sessions/WS-xx` work.
- If a runbook command is simply wrong (stale path, wrong port, changed body
  shape), that's a **doc bug**: fix the doc, bump `Verified:`, don't touch the
  backend. Only a genuine behavior mismatch is a backend bug.
