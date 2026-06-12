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
| WS-05 | Postgres backend for the cloud profile | ⬜ | | | |
| WS-06 | Auth: OIDC/JWT middleware + RBAC org→team→site | ⬜ | | | |
| WS-07 | Tenancy: org/site → awaken `ScopeId` | ⬜ | | | |
| WS-08 | Outbound MCP adapter (BMS tools to external agents) | ⬜ | | | |
| WS-09 | Scoped zenoh session per driver + reference driver binary | ⬜ | | | |
| WS-10 | Write ack/backpressure protocol + bounded buffers | ⬜ | | | |

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
