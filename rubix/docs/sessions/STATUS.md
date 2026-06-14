# Rubix Backend Build — Workstream Queue

The unattended build queue for the rubix backend. Driven by
[_ORCHESTRATION.md](./_ORCHESTRATION.md). Each row is a workstream (WS) with a spec doc in this
directory. Status legend: ⬜ pending · 🔵 in-progress · ✅ done · ⛔ blocked (see TODOs.md).

Branch: **`new-rubix`**. Gap source: [../../STATUS.md](../../STATUS.md) "Not started / remaining";
target: [../../STACK-DEISGN.md](../../STACK-DEISGN.md); scope: [../SCOPE.md](../SCOPE.md).

Queue order is dependency order — earlier rows ship contracts later rows build on. The build is
greenfield: WS-01 stands up the Cargo workspace and the SurrealDB store boundary; every later WS
finds its dependencies already committed.

| # | Workstream | Status | Started | Finished | Commit |
| --- | --- | --- | --- | --- | --- |
| WS-01 | Workspace foundation + SurrealDB embedded core store | ✅ | 2026-06-14T14:59:44Z | 2026-06-14T15:12:09Z | 8d2a135a |
| WS-02 | Generic record model + tag graph | ✅ | 2026-06-14T15:20:13Z | 2026-06-14T15:41:00Z | 3f7a4c43 |
| WS-03 | Identity + scoped read session | ✅ | 2026-06-14T15:40:14Z | 2026-06-14T15:55:00Z | a4212379 |
| WS-04 | Capability grants (app-enforced authz) | ✅ | 2026-06-14T16:05:13Z | 2026-06-14T16:30:00Z | 59d300d4 |
| WS-05 | Command gate + audit + correlation id | ✅ | 2026-06-14T16:20:17Z | 2026-06-14T17:05:00Z | a0e59b67 |
| WS-06 | Undo/redo reversible change records | ✅ | 2026-06-14T16:35:16Z | 2026-06-14T16:46:18Z | d8b8721a |
| WS-07 | Event bus: in-process + live-query data-change | ⬜ | | | |
| WS-08 | Tracing spans on the bus | ⬜ | | | |
| WS-09 | DataFusion query surface over SurrealDB + vector search | ⬜ | | | |
| WS-10 | Datasource connector framework + Postgres connector | ⬜ | | | |
| WS-11 | Rhai rules / insights runtime | ⬜ | | | |
| WS-12 | Zenoh ingestion + pre-processing | ⬜ | | | |
| WS-13 | Extensions as scoped principals | ⬜ | | | |
| WS-14 | Edge/cloud profiles (features + runtime config) | ⬜ | | | |
| WS-15 | Edge↔cloud sync shipper over Zenoh | ⬜ | | | |
| WS-16 | Transport: axum HTTP + JSON-RPC + WS bridge + OpenAPI + prefs | ⬜ | | | |

## Dependency notes
- **WS-01** is the root: workspace, `rubix-core` error enum + ids, `rubix-store` SurrealDB boundary,
  minimal `rubix-server` AppState. Everything depends on it.
- **WS-02** (records + tag graph) is the schemaless domain every read/write operates on.
- **WS-03→WS-06** are the gate, in order: scoped read session → capability grants → command
  gate+audit → undo. WS-05 depends on WS-03/04 (identity + grants); WS-06 depends on WS-05's capture.
- **WS-07** (event bus) needs WS-03's scoped session (data-change plane is permission-filtered);
  **WS-08** (tracing) emits on WS-07's bus, correlated by WS-05's correlation id.
- **WS-09** (DataFusion) reads over WS-01's store; **WS-10** connectors plug into WS-09's surface.
- **WS-11** (Rhai) consumes WS-09 window values, records via WS-05 gate, publishes on WS-07 bus,
  emits WS-08 span trees.
- **WS-12** (Zenoh ingest) scopes by WS-04 capability and persists WS-02 records edge-partitioned.
- **WS-13** (extensions) are WS-03 principals with WS-04 grants on both planes.
- **WS-14** (profiles) gates WS-10 Postgres + cloud namespace-per-tenant behind a feature seam.
- **WS-15** (sync) ships WS-02/audit records over Zenoh with the partition+ownership conflict model.
- **WS-16** (transport) wires every crate into the axum binary + JSON-RPC + WS live-query bridge +
  OpenAPI, and adds `rubix-prefs`.

## Loop log
<!-- The loop appends one line per wake here: <utc> <action> (spawned WS-xx / gated WS-xx ✅ / blocked WS-xx ⛔ / idle). -->
- 2026-06-14T14:59:44Z spawned WS-01
- 2026-06-14T15:15:11Z gated WS-01 ✅ (tests green, clippy clean, commit 8d2a135a)
- 2026-06-14T15:20:13Z spawned WS-02
- 2026-06-14T15:36:07Z gated WS-02 ✅ (tests green, clippy clean, commit 3f7a4c43)
- 2026-06-14T15:40:14Z spawned WS-03
- 2026-06-14T16:00:49Z gated WS-03 ✅ (tests green, clippy clean, commit a4212379)
- 2026-06-14T16:05:13Z spawned WS-04
- 2026-06-14T16:15:50Z gated WS-04 ✅ (tests green, clippy clean, commit 59d300d4)
- 2026-06-14T16:20:17Z spawned WS-05
- 2026-06-14T16:35:16Z gated WS-05 ✅ (tests green, clippy clean, commit a0e59b67)
- 2026-06-14T16:35:16Z spawned WS-06
- 2026-06-14T16:46:18Z gated WS-06 ✅ (tests green, clippy clean, commit d8b8721a)
