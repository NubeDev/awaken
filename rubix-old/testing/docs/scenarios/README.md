# Scenarios — Cross-Feature Golden Paths

> Feature docs ([../features/](../features/)) test one feature in isolation.
> Scenarios chain them into the user-visible flows that must *always* work — the
> regression set. **Status: verified on `rubix-gaps` 2026-06-13** — S1 (Phase 0),
> S2/S3 live against a real OpenAI run (`gpt-4o-mini`), S4 via the cross-feature
> `tenancy`/`scoped`/`bus` integration tests, S5 by two clean-DB runs. Every gate
> green; no backend bug found at the scenario level (the feature-level fixes already
> landed). Per-gate evidence is recorded under each scenario below.

Each scenario is a numbered, top-to-bottom runbook with ✅ gates. A scenario is
green only when every gate holds. On first failure → capture + triage + fix
([../feedback-loop/](../feedback-loop/)).

---

## S1 — Sim to History (the headline path)

The end-to-end proof the edge works: a spawned driver becomes stored, queryable data.

1. Stack up + sim driver ([../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md)).
2. Provision `nube/hq/ahu-3/temp` ([../features/POINTS_PRIORITY_ARRAY.md](../features/POINTS_PRIORITY_ARRAY.md)).
3. Supervisor spawns the sim; it attaches and publishes `cur`
   ([../features/DRIVER_SUPERVISOR.md](../features/DRIVER_SUPERVISOR.md),
   [../features/ZENOH_BUS.md](../features/ZENOH_BUS.md)).
4. ✅ The point's `cur_value` oscillates in `[19.0, 23.0]`; `his` accumulates;
   `SELECT * FROM points_cur` shows the keyexpr + value
   ([../features/QUERY_AND_ROLLUP.md](../features/QUERY_AND_ROLLUP.md)).

## S2 — Rule Board → Spark → Agent (the closed loop)

Builds on S1's flowing data. The signature rubix capability.

1. A stored rule board evaluates the live data and `emit_spark`s a finding
   ([../features/BOARDS_REFLOW.md](../features/BOARDS_REFLOW.md)).
2. The spark publishes on `{org}/{site}/spark/{rule}/{id}`; the dispatcher activates
   an agent run on a spark-keyed thread
   ([../features/AI_TOOLS_AND_AGENT.md](../features/AI_TOOLS_AND_AGENT.md)).
3. ✅ The board emits the spark; a `runs` entry appears with `origin: dispatch`; its
   gated tool calls investigate (and act at/below the ceiling). Requires
   `RUBIX_AI=1` + `RUBIX_AI_DISPATCH=1`.

> **Verified live 2026-06-13** (real OpenAI run). A stored `hi-temp-rule` board
> (`read_point → emit_spark`, `kind:"manual"`) run via `/boards/{slug}/run` emitted a
> spark (`rule:hi-temp`, `site_id` resolved); the board's `emit_spark` published it on
> `nube/hq/spark/hi-temp/{id}` (the bus path needs `with_bus` on the board access,
> which the HTTP/scheduler paths wire); the dispatcher activated an agent run
> `origin:"dispatch"` on thread `spark-<id>`, completed (`steps:8`). The whole
> Boards→Bus→Dispatch→Agent loop closed. **Note:** the dispatched run is async — list
> `/runs` *after* the log shows `dispatch: agent run completed`, or you race it (it
> reads as empty until the run settles).

## S3 — HITL Escalation

Proves the human-in-the-loop safety band on top of the agent.

1. Prompt the agent to write at an escalation-band priority (e.g. 8).
2. ✅ The run suspends (`awaiting_approval` + `run_id`), the store is untouched;
   `resume` applies the write (one-shot, 409 on re-resume); `cancel` drops it.
   Below-floor writes are denied outright.
   ([../features/AI_TOOLS_AND_AGENT.md](../features/AI_TOOLS_AND_AGENT.md))

> **Verified live 2026-06-13** (real OpenAI run). prio-8 write → `awaiting_approval` +
> `run_id`, slot-8 null (store untouched); `resume` → `effective:18.0`, slot-8=18.0;
> second `resume` → `409`; a fresh prio-6 suspend `cancel` → `204` and slot-6 stays
> null. Deny band (below-floor hard refusal, no run row) proven exhaustively in the
> feature doc's boundary sweep (floor=5: p4 deny / p5,p12 escalate / p13 commit).
> **Doc note:** `cancel` returns `204 No Content` (empty body).

## S4 — Tenant Isolation

Proves the scope model end to end on a two-site setup.

1. Provision `nube/hq` and `nube/hq2`. Bind a run/principal to `nube/hq`.
2. ✅ The scoped run reads/writes only under `nube/hq`; `nube/hq2` is refused at the
   tool boundary; a scoped `/query` reads only its own rows; the bus answers only
   for owned sites. (Capability `covers` is the single primitive — sibling
   `nube/hq2` is *not* covered by `nube/hq`.)

> **Verified 2026-06-13** via the cross-feature integration tests (deterministic,
> scripted agent — the correct proof since edge HTTP has no principal to bind a scope
> to, so cross-tenant enforcement only triggers on a *bound* scope):
> `rubix-server` `api_tests::tenancy` (2/2) — a site-A-scoped run is refused commanding
> site B at the **tool boundary** on both the dispatch and chat paths, with in-scope
> controls landing; `rubix-query` `tests/scoped` (5/5) — a scoped session sees only its
> tenant and can't name a sibling even via an explicit predicate; `api_tests::bus`
> `write_query_for_unowned_site_gets_no_reply` — the bus stays silent for an unowned
> site. All four sub-claims hold.

## S5 — Determinism / Regression

The repeatable baseline for catching regressions.

1. Clean store; fixed sim config (`baseline`/`amplitude`/`period_secs`); run for a
   fixed window.
2. ✅ Re-running from a clean DB reproduces the same `his` **value cycle** (the sim's
   triangle wave is deterministic — no RNG) and the same rollup aggregates. Any
   drift is a regression to triage.

> **Verified 2026-06-13** by two clean-DB runs (sim `period_secs:1`, `baseline:21`,
> `amplitude:2`). Both reproduced the deterministic 12-step triangle cycle
> `[19.0, 19.7, 20.3, 21.0, 21.7, 22.3, 23.0, 22.3, 21.7, 21.0, 20.3, 19.7]` — identical
> value set, identical step-to-step transitions. The value is a pure function of an
> incrementing `step` counter (`sample(step)` in `rubix-driver-sim/src/simulate.rs`,
> `step % 12`), no wall-clock, no RNG.
>
> **Nuance — assert the cycle, not the absolute first element.** The two runs started
> the cycle at *different* offsets (run 1 at `19.7`, run 2 at `20.3`). That is **not**
> non-determinism in the wave: it's a race between the sim's first ticks and when the
> point is *provisioned* (history only lands once the point exists). So the stable
> regression invariant is "the recorded values are a contiguous rotation of the cycle
> and every consecutive pair is a valid cycle transition," **not** "his[0] == 19.0".
> A scripted scenario should provision the point *before* the sim starts (or compare
> rotation-invariantly) to pin the offset.

---

## Running scenarios

`testing/scripts/run-scenario.sh <S1..S5>` (git-ignored) boots the stack, runs a
scenario's ✅ gates, and auto-captures an evidence bundle on the first ❌ (see
[../feedback-loop/FIX_LOOP.md](../feedback-loop/FIX_LOOP.md) "scripted driver").
S1/S5 run the live data path directly (no key needed). S2/S3/S4 need a live agent or
are proven by the cross-feature integration suites this doc cites, so the script runs
those suites as their gate — S2 falls back to the scripted `dispatch` test unless
`RUBIX_AI=1` + `OPENAI_API_KEY` are set. `KEEP_STACK=1` leaves the server up for a
hand-driven follow-up. You can still run any runbook by hand.

## Adding a scenario

Keep them few and high-value — golden paths, not exhaustive coverage (that's the
feature docs' job). Each new scenario: a numbered list of steps, explicit ✅ gates,
and a pointer to the feature docs it composes.
