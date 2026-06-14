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

- **WS-01 — Workspace foundation + SurrealDB embedded core store.** Standalone
  `rubix/` Cargo workspace (`rubix-core`, `rubix-store`, `rubix-server`);
  `rubix-core` ids + project error enum with `.context()` chaining +
  `CorrelationId` + `RuntimeConfig`; `rubix-store` embedded SurrealDB boundary
  (kv-mem / kv-surrealkv, namespace+database bootstrap, schema-init seam, health
  probe, durable read/write handle, scoped-session issuance seam);
  `rubix-server` axum binary with `AppState` + `GET /health`;
  `scripts/check-file-size.sh` 400-line guard.

- **WS-02 — Generic record model + tag graph.** `rubix-core` `record` module
  (schemaless `Record` with free-form JSON content + create/update timestamps;
  `create`/`read`/`update`/`delete` over SurrealQL) and `tag` module (`Tag` CRUD;
  `attach`/`detach` the `record→tagged→tag` edge; `find_records_by_tags`
  Haystack-style multi-tag set-intersection traversal). `rubix-store::init_schema`
  now declares the `record`/`tag`/`tagged` (relation) tables `SCHEMALESS`.

- **WS-03 — Identity + scoped read session.** `rubix-core` `principal` module
  (one identity model: `Principal` = subject/namespace/kind(`user|extension`)/
  role, owned per the crate map) plus a `list_records` read verb whose visible
  set is decided by the session's permissions. New `rubix-gate` crate: the read
  enforcement point. `define_gate_schema` declares the `principal` record-access
  method (`DEFINE ACCESS principal ON DATABASE TYPE RECORD` with a `SIGNIN`
  query) and `DEFINE TABLE OVERWRITE record … PERMISSIONS FOR select WHERE
  namespace = $auth.namespace` (SurrealDB-native row-level read scope).
  `authenticate` resolves a `PrincipalToken` to a `Principal`; `provision_principal`
  registers an identity on the root handle; `issue_scoped_session` clones the
  store connection (a fresh session over the same datastore) and signs it in as
  the principal, so reads via `read_records_on_session` / `read_record_on_session`
  are confined to the principal's namespace by the engine — a cross-namespace
  read returns empty/denied with no app filter (contracts #1 and #2).

---

## Not started / remaining (per STACK-DEISGN.md)

### Foundation
- [x] Cargo workspace + file-size guard + project error enum (`rubix-core`).
- [x] SurrealDB embedded store: connection/namespace bootstrap, schema init,
      health, scoped-session issuance seam (`rubix-store`).
- [x] Minimal `rubix-server` binary + `AppState` (health route) to host later wiring.

### Domain model
- [x] Generic record model — schemaless document records, no fixed ontology.
- [x] Tag graph — `record→tagged→tag` graph edges + tag CRUD + tag-filter queries.

### Access & policy gate
- [x] Identity model — users and extensions as scoped principals (one model).
- [x] Scoped read session — gate-issued SurrealDB session, row-level perms.
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

## Environment variables

| Var | Default | Used by | Meaning |
| --- | --- | --- | --- |
| `RUBIX_NAMESPACE` | `rubix` | `rubix-server` | SurrealDB namespace to bootstrap/use. |
| `RUBIX_DATABASE` | `main` | `rubix-server` | SurrealDB database to bootstrap/use. |
| `RUBIX_DATA_DIR` | `rubix-data` | `rubix-server` | SurrealKV file-backed data directory. |
| `RUBIX_BIND` | `127.0.0.1:8080` | `rubix-server` | HTTP listen address. |

---

## Build & test

```
cd rubix
cargo test --workspace
cargo clippy --workspace --all-targets
```
