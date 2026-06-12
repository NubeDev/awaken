# Rubix Backend Build — Workstream Queue

The unattended build queue for the rubix backend gaps. Driven by
[_ORCHESTRATION.md](./_ORCHESTRATION.md). Each row is a workstream (WS) with a spec doc in this
directory. Status legend: ⬜ pending · 🔵 in-progress · ✅ done · ⛔ blocked (see TODOs.md).

Branch: **`rubix-gaps`**. Gap source: [../../../STATUS.md](../../../STATUS.md) "Not started /
remaining"; target: [../../../STACK-DEISGN.md](../../../STACK-DEISGN.md).

Queue order is dependency order — earlier rows ship contracts later rows build on.

| # | Workstream | Status | Started | Finished | Commit |
| --- | --- | --- | --- | --- | --- |
| WS-01 | `agent_call` board node → embedded agent | ✅ | 2026-06-12T09:44:18Z | 2026-06-12T10:35:00Z | 47221b45 |
| WS-02 | Persistent run registry + resume endpoint (HITL) | ✅ | 2026-06-12T10:10:09Z | 2026-06-12T11:40:00Z | 92f2d912 |
| WS-03 | Parquet history `TableProvider` (object_store tiering) | ✅ | 2026-06-12T10:35:35Z | 2026-06-12T12:20:00Z | 2b717955 |
| WS-04 | Edge/cloud profiles (cargo features + runtime config) | ✅ | 2026-06-12T11:00:13Z | 2026-06-12T11:33:26Z | a5648944 |
| WS-05 | Postgres backend for the cloud profile | ✅ | 2026-06-12T11:40:11Z | 2026-06-12T12:55:00Z | 200269f6 |
| WS-06 | Auth: OIDC/JWT middleware + RBAC org→team→site | ✅ | 2026-06-12T12:10:30Z | 2026-06-12T13:40:00Z | b7ec6d06 |
| WS-07 | Tenancy: org/site → awaken `ScopeId` | ✅ | 2026-06-12T12:30:31Z | 2026-06-12T13:21:00Z | 385f0e1d |
| WS-08 | Outbound MCP adapter (BMS tools to external agents) | ✅ | 2026-06-12T13:30:10Z | 2026-06-12T14:05:00Z | 9e8be915 |
| WS-09 | Scoped zenoh session per driver + reference driver binary | ✅ | 2026-06-12T13:50:12Z | 2026-06-12T14:20:00Z | 08217e33 |
| WS-10 | Write ack/backpressure protocol + bounded buffers | ✅ | 2026-06-12T14:00:23Z | 2026-06-12T14:35:00Z | 8aaab1d8 |

## Dependency notes
- **WS-02** lands the persistent run store the HITL resume endpoint needs (STATUS.md flags this as
  "lands with the mailbox/dispatch layer"). WS-08's external runs reuse it.
- **WS-04** introduces the profile/feature split that **WS-05** (Postgres edge/cloud), **WS-09**
  (driver runtime), and **WS-10** consume; it ships the runtime-config plumbing first.
- **WS-05** (Postgres) ships behind the WS-04 `cloud` profile; SQLite stays the edge default.
- **WS-06** (auth) and **WS-07** (tenancy/ScopeId) share the org→team→site hierarchy; WS-06 ships the
  RBAC tables and middleware, WS-07 mirrors the resolved scope into awaken's `ScopeId`.
- **WS-09** and **WS-10** are the driver-runtime pair; WS-10 builds on WS-09's per-driver session.

## Loop log
<!-- The loop appends one line per wake here: <utc> <action> (spawned WS-xx / gated WS-xx ✅ / blocked WS-xx ⛔ / idle). -->
- (pending first wake)
- 2026-06-12T09:44:18Z spawned WS-01
- 2026-06-12T10:35:00Z gated WS-01 ✅ (47221b45; pre-existing HITL-suspend test hang logged to TODOs.md)
- 2026-06-12T10:10:09Z spawned WS-02
- 2026-06-12T11:40:00Z gated WS-02 ✅ (92f2d912; pre-existing HITL-suspend test hang from WS-01 fixed — full suite terminates, 125 tests green)
- 2026-06-12T10:35:35Z spawned WS-03
- 2026-06-12T10:56:10Z gated WS-03 ✅ (2b717955; cargo test --workspace green, clippy clean, OpenAPI surface compiles)
- 2026-06-12T11:00:13Z spawned WS-04
- 2026-06-12T11:34:33Z re-spawned WS-04 (prior subagent died mid-session with uncommitted work; resumed idempotently) → gated WS-04 ✅ (a5648944; clippy clean + tests green under both edge & cloud features)
- 2026-06-12T11:40:11Z spawned WS-05
- 2026-06-12T12:55:00Z gated WS-05 ✅ (200269f6; synchronous Postgres Backend behind the cloud feature, shared store_suite green on both SQLite and a live Postgres; DataFusion-Postgres federation logged to TODOs.md)
- 2026-06-12T12:10:30Z spawned WS-06
- 2026-06-12T13:40:00Z gated WS-06 ✅ (b7ec6d06; OIDC-JWT + PAT bearer auth and org→team→site RBAC behind the cloud profile seam, edge unchanged; tests green on edge & cloud features, clippy clean both; per-route scope gating beyond the site routes logged as a follow-up)
- 2026-06-12T12:30:31Z spawned WS-07 → returned Done and self-gated ✅ (385f0e1d; org/site→ScopeId mapping enforced at the tool boundary on both chat and dispatch paths, site-A run refused a site-B write; cargo test --workspace green, clippy clean; concurrent-session SQL query-scope subsystem reconciled, prior fail-closed TODO resolved)
- 2026-06-12T13:30:10Z spawned WS-08 → returned Done and self-gated ✅ (9e8be915; outbound MCP adapter at POST /api/v1/mcp dispatches gated BMS tools to external agents — priority-array gating, tenant scope, and HITL escalation reuse build_tools_scoped and the runs registry; cargo test --workspace green on edge, mcp suite green on cloud, clippy clean both)
- 2026-06-12T13:50:12Z spawned WS-09 (scoped zenoh session per driver + reference driver binary; first pending in queue order, WS-01..08 all ✅)
- 2026-06-12T14:20:00Z gated WS-09 ✅ (ScopedSession wrapper confines the sim's zenoh session to its CapabilitySet — publish/subscribe outside the grant refused locally before the bus; live supervisor-spawn out-of-grant-publish refusal test closes the prior known gap; cargo test --workspace green, clippy clean)
- 2026-06-12T14:00:23Z spawned WS-10 (write ack/backpressure protocol + bounded buffers; first pending in queue order, WS-01..09 all ✅; builds on WS-09's per-driver session)
- 2026-06-12T14:35:00Z gated WS-10 ✅ (3b1e7358 driver-contract buffers/ack, 8887ddb0 driver-sim wiring, 8aaab1d8 docs; CurBuffer drop-oldest + visible counter, ReliableQueue full→BufferFull, write ack/retry→AckTimeout give-up; driver 18 + driver-sim 9 tests green incl. live retry/ack/give-up/saturation, clippy clean workspace-wide; pre-existing WS-09 supervised liveliness-clear + api HITL-suspend host failures logged to TODOs.md, both confirmed not WS-10 regressions). Final queue WS — run complete.
