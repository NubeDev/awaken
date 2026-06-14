# STACK DESIGN — Rubix edge-to-cloud data platform

The build target for the rubix backend (`rubix/`, a standalone Cargo workspace,
not part of the awaken workspace). The **scope authority** is
[docs/SCOPE.md](docs/SCOPE.md); this doc translates that scope into a concrete
crate map, the cross-cutting contracts each crate must honor, and the key
design decisions that are otherwise left implicit. Where this doc and SCOPE.md
disagree, SCOPE.md wins and this doc is corrected.

The platform is a generic, AI-ready, edge-to-cloud data processing platform on
**SurrealDB** as the single store and brain: ingest from any source, transform
in flight, run rules and emit insights, serve a unified query surface — the same
binary on a single edge device or in a multi-tenant cloud. The domain is not
baked in; structure comes from tagging on a graph.

## Layout discipline

Every crate obeys [docs/FILE-LAYOUT.md](docs/FILE-LAYOUT.md): one verb per file,
folder-of-verbs over file-of-nouns, ≤400 lines hard / ~100 typical, names are
concepts never shapes (no `utils.rs`/`helpers.rs`/`common.rs`/`types.rs`).
`mod.rs` is a barrel only. Tests mirror the source tree.

## Crate map

| Crate | Role | Owns |
| --- | --- | --- |
| `rubix-core` | Domain model + shared contracts | ids, the project error enum, generic records, the tag graph (`record→tagged→tag`), correlation id, principal identity types |
| `rubix-store` | SurrealDB embedded store boundary | connection/namespace bootstrap, schema init, scoped-session issuance, the durable read/write boundary |
| `rubix-gate` | Access & policy gate | command path (authz → audit/undo capture → correlation id → apply), capability grants, scoped read sessions |
| `rubix-bus` | Event bus | in-process tokio control plane + SurrealDB live-query data-change plane, permission-filtered per principal |
| `rubix-trace` | Tracing | correlated spans emitted on the bus, bounded/sampled retention in SurrealDB |
| `rubix-query` | DataFusion query/compute | SurrealDB `TableProvider`, unified query surface, vectorized window aggregation, vector/semantic search |
| `rubix-datasource` | Pluggable connectors | connector trait → DataFusion `TableProvider`, Postgres connector (SurrealDB is native/default) |
| `rubix-rules` | Rhai rules/insights | embedded engine, composable rules, decision recorded back to SurrealDB + published as a data-change event, span tree per eval |
| `rubix-ingest` | Ingestion/pre-process | Zenoh subscribe (key-space scoped), decimate/filter/enrich, persist edge-partitioned append-only |
| `rubix-ext` | Extensions as principals | service-account model, JSON-RPC control plane, Zenoh data plane, capability scoping |
| `rubix-sync` | Edge↔cloud sync | app-level shipper over Zenoh, append-only data partition by edge id, config-plane ownership + LWW + audit tiebreak |
| `rubix-prefs` | Preferences | per-user units (metric/imperial) + datetime display formatting |
| `rubix-server` | Binary + transport | axum HTTP, JSON-RPC control, WebSocket live-query bridge, OpenAPI, edge/cloud profile selection, `AppState` wiring |

## Load-bearing contracts (every crate honors these)

1. **Two enforcement points, one identity.** *Commands* (mutations) cross the
   gate (`rubix-gate`): authenticate → check capability grants → capture
   before/after for audit+undo → mint/carry the correlation id → apply. *Reads*
   (including live queries) run on a gate-issued **scoped SurrealDB session** and
   are enforced by SurrealDB row-level permissions — never proxied per message.
2. **Two authz layers (do not conflate).** Data-record perms are SurrealDB-native
   (scoped session). Capability grants (register a datasource, invoke a rule,
   publish ingest, query an external source, subscribe a Zenoh key-space) are
   app-enforced by the gate. Both key off the same principal.
3. **Correlation id is the linchpin.** Minted at the gate (principal actions) or
   at ingest (data), carried on every bus event, stamped into audit records, undo
   change records, and spans.
4. **Audit ≠ undo ≠ trace.** Audit: append-only, immutable, long retention. Undo:
   mutable per-principal+resource stack, definitions/config only, applied through
   the gate. Trace: append-only, bounded/sampled. All three derive from the two
   chokepoints (gate + bus), not bolted on.
5. **Append-only data, edge-partitioned.** Readings/insights/audit/traces are
   written into a partition keyed by the edge identity — two edges never write the
   same records, so reconciliation is ordering + dedup, not merge.
6. **SurrealDB does as much as possible.** Document, graph, vector, time-series,
   geospatial, auth, tenancy, live queries in one engine. DataFusion sits above
   only for cross-datasource unification and heavy vectorized aggregation.

## Key decisions (resolve SCOPE open questions enough to build)

- **Embedded SurrealDB engine.** `kv-mem` for tests; the file-backed default is
  SurrealKV (`kv-surrealkv`) selected by config. Durability/ops tuning is a
  follow-up, not a blocker (SCOPE open question 4).
- **Edge = single namespace; cloud = namespace-per-tenant.** Same binary, cargo
  feature (`edge` default / `cloud`) + `RUBIX_PROFILE` runtime select. Edge gate
  resolves to the one tenant automatically.
- **Sync is app-level over Zenoh**, not DB replication. Data plane append-only by
  edge partition; config plane ownership + last-write-wins with the audit log as
  tiebreak (SCOPE open question 1 — starting position, CRDT only if required).
- **Rhai owns the decision; DataFusion owns the data.** Time-window math is
  computed in DataFusion and fed to Rhai, which makes the decision and records the
  insight. Heavy aggregation never lives in Rhai.

## Build & test

```
cd rubix
cargo test --workspace
cargo clippy --workspace --all-targets
```

Edge is the default build. Cloud-only paths (Postgres, namespace-per-tenant) sit
behind the `cloud` cargo feature and fail closed when their backend is absent.
