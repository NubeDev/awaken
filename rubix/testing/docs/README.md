# Rubix Testing & Feedback Suite

> **You are an AI session picking this up cold. Read this file top to bottom, then
> jump to the doc that matches your task.** Every doc is a self-contained runbook:
> exact commands, exact files, exact pass/fail checks. Nothing here assumes prior
> context beyond what is written down.

Rubix is a BMS/EMS backend: a standalone Cargo workspace (`rubix/`, *not* part of
the awaken workspace). It models sites/equips/points with a 16-level BACnet
priority array, runs a zenoh data plane, answers SQL over SQLite via DataFusion,
evaluates reflow control/rule boards, and embeds an awaken AI agent that can read,
write (priority-gated), query, and run boards.

This suite does three things:

1. **Stand up rubix with realistic, live data** — the `rubix-driver-sim`
   reference driver, spawned by the supervisor, publishes simulated `cur` samples
   over zenoh; the server lands them in SQLite history on ingest.
2. **Exercise each feature** end-to-end with scripted runbooks (points + priority
   array, zenoh bus, query + rollup, reflow boards, AI tools/agent, the driver
   supervisor).
3. **Close a feedback loop** — capture a consistent evidence bundle (logs,
   server state, row counts, the failing request) so an AI session can diagnose
   and fix rubix, then re-run and confirm the fix.

---

## The data path (read this once, it explains everything)

```
 rubix-driver-sim ──zenoh put──▶  zenoh mesh  ──▶  rubix-server bus  ──▶  SQLite store
 (spawned by the                 (peer/router)    (cur sub + write/his    (points.cur_value
  supervisor, scoped                              queryables, scoped       + his rows)
  to a keyexpr grant)                             to owned sites)              │
                                                                              ▼
                                          /api/v1/query (DataFusion) ─▶ boards / agent / sparks
```

Key facts that shape all testing (code-grounded 2026-06-13, see
[reference/ARCHITECTURE.md](reference/ARCHITECTURE.md) for file:line evidence):

- **No bearer auth on the edge profile by default.** Auth is OIDC-JWT + PAT, and
  it is **off** unless `RUBIX_OIDC_ISSUER`/`RUBIX_OIDC_JWKS` are set (edge) — the
  cloud profile *requires* it. So local testing hits the API with no token. Don't
  copy nexus's cookie/CSRF flow; rubix has neither.
- **Everything is keyexpr-scoped, not RLS-scoped.** Identity is the path
  `{org}/{site}/{equip-path}/{point}`. Capabilities, query scope, and agent tool
  scope all confine to an `{org}/{site}` prefix via the *same* path-boundary match
  (`Capability::covers`): `nube/hq` covers `nube/hq/ahu-3` but **not** `nube/hq2`.
- **Writes always go through the 16-level priority array, never raw.** Lower level
  number wins (1 = highest authority, 16 = lowest). The agent is capped at
  `RUBIX_AI_MIN_PRIORITY` (default 13); writes above that ceiling **suspend for
  human approval**; writes below `RUBIX_AI_ESCALATION_FLOOR` are hard-refused.
- **SQLite is the store on the edge** (`RUBIX_DB`, default `rubix.db`); Postgres is
  a cloud-feature option. DataFusion reads SQLite read-only.
- **Feature-gated subsystems return 503 when off**, they don't 404: `/query`
  (`RUBIX_QUERY`), `/agent/chat` (`RUBIX_AI`, default **off**), `/his/flush`
  (`RUBIX_HIS_PARQUET`). Zenoh, supervisor, scheduler, dispatcher degrade silently.

---

## Map of this suite

| Path | What it is | When to read it |
|------|-----------|-----------------|
| [`TODO.md`](TODO.md) | The ordered work queue — what to verify, in what order | **Read first to know where to start.** |
| [`00_setup/QUICKSTART.md`](00_setup/QUICKSTART.md) | One-page bring-up: server, zenoh, sim driver, first live `cur` value | First hands-on step (TODO Phase 0). |
| [`00_setup/STACK.md`](00_setup/STACK.md) | Every process, port, env var, and how to tear down cleanly | When something won't start |
| [`00_setup/SIM_DRIVER.md`](00_setup/SIM_DRIVER.md) | The reference driver: manifest, capabilities, the `cur` it publishes, determinism, knobs | When you need specific data shapes |
| [`features/`](features/) | One runbook per feature (see below) | When testing/fixing that feature |
| [`scenarios/`](scenarios/) | Cross-feature golden-path scripts ("sim → his → board → spark → agent") | End-to-end / regression |
| [`feedback-loop/`](feedback-loop/) | Evidence capture + triage + fix loop for AI sessions | When something is broken |
| [`reference/ARCHITECTURE.md`](reference/ARCHITECTURE.md) | Grounded system map with file:line citations | When a doc's claim looks stale |
| [`reference/API_CHEATSHEET.md`](reference/API_CHEATSHEET.md) | curl-ready endpoint list, no-auth + PAT paths | Constantly |

### Feature runbooks (`features/`)

| Doc | Covers | Status |
|-----|--------|--------|
| [ENTITY_CRUD_AND_TENANCY.md](features/ENTITY_CRUD_AND_TENANCY.md) | full CRUD lifecycle of sites/equips/points/boards/widgets/sparks (gap-detects the missing `Update` verb); org/site tenant isolation | scaffold (2026-06-13) — drives [design/crud-and-tenancy.md](../../docs/design/crud-and-tenancy.md) |
| [POINTS_PRIORITY_ARRAY.md](features/POINTS_PRIORITY_ARRAY.md) | sites/equips/points CRUD; the 16-level write/relinquish/cur path; history | verified (`d48dd8b2`, 2026-06-13) |
| [ZENOH_BUS.md](features/ZENOH_BUS.md) | live `cur` pub/sub, `**/write` + `**/his/**` queryables, owned-site scoping, spark publish | verified (2026-06-13) — `api_tests::bus` 7/7 |
| [QUERY_AND_ROLLUP.md](features/QUERY_AND_ROLLUP.md) | DataFusion SQL over SQLite, `points_cur` view, `/his/rollup` buckets + aggregates, tenant scope | verified (2026-06-13) — fixed `/query` read-only guard |
| [BOARDS_REFLOW.md](features/BOARDS_REFLOW.md) | reflow node palette, board JSON, inline `/boards/run`, stored/versioned boards, scheduler | verified (`bdbd01f7`, 2026-06-13) |
| [AI_TOOLS_AND_AGENT.md](features/AI_TOOLS_AND_AGENT.md) | the four tools, priority gating, HITL escalation/runs, spark dispatch, MCP | verified (`bdbd01f7`, 2026-06-13) |
| [DRIVER_SUPERVISOR.md](features/DRIVER_SUPERVISOR.md) | manifest loading, spawn/liveliness/backoff, capability scoping, fail-closed | verified (`bdbd01f7`, 2026-06-13) |
| [DATASOURCE.md](features/DATASOURCE.md) | read-only native-SQL passthrough to external DBs (Timescale/PG): bound params, single-statement, caps (lenient/strict), named queries, registry, describe | library-verified (2026-06-13) — core only, **not integrated** |
| [RULES_ENGINE.md](features/RULES_ENGINE.md) | sandboxed Rhai rule layer over a DataFusion engine: curated vectorized primitives, no-row-explosion invariant, sandbox kill-switches, `rule()` composition (cycle/depth-bounded), finding result; **integrated** — board `rule` node + org-scoped stored-rule store | verified (live) (2026-06-13) — `rubix-rules` 71/71 + L1–L3 live, clippy clean |

> **verified** = run live against a stack and the commands/outputs confirmed.
> **library-verified** = a standalone library crate, fully unit-tested + clippy-clean
> in isolation, but not yet wired into a running stack (its gates are `cargo test`,
> not live-stack runbook steps). Flips to **verified** once integrated.
> **partial** = some gates verified live, the rest still scaffold.
> **scaffold** = structure + acceptance criteria written from the code; commands
> to be confirmed live as we knock off each feature one at a time. Each scaffold
> lists exactly what "done" looks like so the next session knows when to stop.

---

## How to work in this suite (the operating contract)

0. **Read [`TODO.md`](TODO.md)** — it orders the work; start at the first unchecked
   box (Phase 0).
1. **Bring up the stack** via [`00_setup/QUICKSTART.md`](00_setup/QUICKSTART.md).
   Confirm the health checks pass before doing anything else.
2. **Pick a feature or scenario doc.** Run its steps top to bottom.
3. **If a step fails**, do not guess. Go to
   [`feedback-loop/CAPTURE.md`](feedback-loop/CAPTURE.md), produce the evidence
   bundle, then follow [`feedback-loop/TRIAGE.md`](feedback-loop/TRIAGE.md).
4. **When you fix rubix**, re-run the failing step and the relevant scenario;
   record the before/after in the feature doc's "Known issues / fixes" section.
5. **If a doc's stated fact is wrong** (drifted code), fix the doc first, bump its
   `Verified:` line, then proceed — same discipline as the `WS-xx` scope docs.

Each doc carries a `> Verified: <commit> on <date>` header. Treat anything older
than the current branch tip as unverified — re-grep before trusting it.

---

## Conventions used throughout

- Commands assume CWD `rubix/` unless a doc says otherwise.
- Commands prefer the `Makefile` targets (`make dev-be`, `make test`, `make kill`).
  The binary is `rubix` (`cargo run --bin rubix`); the crate is `rubix-server`.
- Env defaults: the server binds `RUBIX_ADDR`, which `make` sets to
  `127.0.0.1:8088` (so the UI proxy reaches it); the bare binary's own default is
  `0.0.0.0:8080`. SQLite at `rubix.db` (`RUBIX_DB`), zenoh peer mode (no router for
  a single node). The UI dev server runs on `5180`. Overridable — see
  [`00_setup/STACK.md`](00_setup/STACK.md).
- `$BASE` = `http://127.0.0.1:8088` (the `make` default). No token on edge unless OIDC is configured;
  where a PAT is needed the cheatsheet shows how to mint one (`POST /api/v1/tokens`).
- Evidence lands in `testing/.evidence/<scenario>/<timestamp>/` (git-ignored).
- ✅ / ❌ checkboxes in runbooks are literal pass/fail gates — a scenario is green
  only when every ✅ holds.
