# Rubix Platform — Build Status

Status of the rubix edge-to-cloud data platform backend (`rubix/`, a standalone
Cargo workspace, not part of the awaken workspace). Measured against
[STACK-DEISGN.md](STACK-DEISGN.md) and [docs/SCOPE.md](docs/SCOPE.md). Reflects
the code as reviewed, not intent.

**State:** greenfield. No Cargo workspace yet — only `docs/`. The build is driven
unattended from [docs/sessions/STATUS.md](docs/sessions/STATUS.md) (the workstream
queue) per [docs/sessions/_ORCHESTRATION.md](docs/sessions/_ORCHESTRATION.md).

---

## Done

Nothing yet. The first workstream (WS-01) stands up the workspace.

---

## Not started / remaining (per STACK-DEISGN.md)

### Foundation
- [ ] Cargo workspace + file-size guard + project error enum (`rubix-core`).
- [ ] SurrealDB embedded store: connection/namespace bootstrap, schema init,
      health, scoped-session issuance seam (`rubix-store`).
- [ ] Minimal `rubix-server` binary + `AppState` (health route) to host later wiring.

### Domain model
- [ ] Generic record model — schemaless document records, no fixed ontology.
- [ ] Tag graph — `record→tagged→tag` graph edges + tag CRUD + tag-filter queries.

### Access & policy gate
- [ ] Identity model — users and extensions as scoped principals (one model).
- [ ] Scoped read session — gate-issued SurrealDB session, row-level perms.
- [ ] Capability grants — app-enforced authz for cross-plane (non-record) actions.
- [ ] Command gate — every mutation through the gate; `RETURN BEFORE` capture.
- [ ] Audit log — append-only, immutable, correlation-id stamped.
- [ ] Undo/redo — reversible change records for definitions, applied through gate.

### Event bus
- [ ] In-process tokio control plane.
- [ ] Data-change plane over SurrealDB live queries, permission-filtered.
- [ ] Tracing spans on the bus, bounded/sampled retention.

### Query / compute
- [ ] DataFusion `TableProvider` over SurrealDB + unified `/query` surface.
- [ ] Vectorized time-window aggregation feeding rule decisions.
- [ ] Vector / semantic search surface.
- [ ] Datasource connector framework + Postgres connector.
- [ ] MQTT / REST connectors (follow-up behind the framework).

### Rules / insights
- [ ] Rhai embedded runtime — composable rules, offline, decision → SurrealDB +
      data-change event, span tree per evaluation.

### Ingestion / transport
- [ ] Zenoh ingestion + pre-processing (decimate/filter/enrich), edge-partitioned.
- [ ] Key-space scope resolved once at subscribe (capability decision).

### Extensions
- [ ] Extension principal model + JSON-RPC control plane + Zenoh data plane.

### Platform / deployment
- [ ] Edge/cloud profiles (single binary, cargo features + runtime config).
- [ ] Edge↔cloud sync shipper over Zenoh (append-only partition + config LWW).
- [ ] Preferences (units + datetime).
- [ ] Transport: axum HTTP + JSON-RPC control + WS live-query bridge + OpenAPI.

---

## Build & test

```
cd rubix
cargo test --workspace
cargo clippy --workspace --all-targets
```
