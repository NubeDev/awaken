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

- **WS-04 — Capability grants (app-enforced authz, the second layer).**
  `rubix-gate` `capability` module: a `Capability` enum (datasource-register,
  rule-invoke, ingest-publish, external-query, zenoh-subscribe) with a
  registry (`is_registered`) that is the fail-closed allow-set; a `Grant`
  binding a capability to a principal's subject within its namespace, persisted
  in the `grant` table (declared by `define_gate_schema`, no scoped-session
  `select` perm — grants are app-enforced, read on the store handle). `create_grant`
  / `list_grants` / `revoke_grant` administer grants through the gate with a
  fail-closed authority rule (`may_administer`: only an Admin operating within
  the grantee's own namespace), so no privilege escalation and no cross-namespace
  grant. `check_capability` denies unknown capabilities and missing grants and
  allows only an exact (subject, namespace, capability) match — the second authz
  layer of contract #2, distinct from WS-03's SurrealDB-native row-level layer,
  keyed off the same `Principal`.

- **WS-05 — Command gate + audit + correlation id.** `rubix-gate` `command`
  module: the single write-enforcement point. A `Command` (principal +
  required `Capability` + target id + `Change` = create/update/delete over
  free-form JSON) crosses `apply`, which sequences the gate pipeline —
  `authorize` (the WS-04 `check_capability` grant check, fail closed before any
  write) → `correlate` (mint a fresh `CorrelationId`, or carry an upstream one
  for WS-06 undo replay) → `capture` (run the mutation with SurrealDB
  `RETURN BEFORE`, so the before-image is taken atomically with the write, one
  round trip, no read-before-write) → `append_audit`. The capture's before/after
  pair is the one consumed by audit here and by undo in WS-06 (one capture, two
  consumers). `audit` module: an immutable `AuditRecord` (subject, namespace,
  action, target, before/after summary, correlation id, timestamp) written
  append-only to the `audit` table; `define_audit_schema` declares that table
  with `PERMISSIONS FOR select WHERE namespace = $auth.namespace` and
  `FOR create, update, delete NONE`, so immutability is engine-enforced — the gate
  appends on the root/owner handle (the only session past `NONE`), and a scoped
  principal's `UPDATE`/`DELETE` is refused by SurrealDB. Contracts #1, #3, #4.

- **WS-07 — Event bus: in-process + live-query data-change.** New `rubix-bus`
  crate, the eventing spine's first two planes (SCOPE "Event bus"). `inprocess`
  module: a cloneable `ControlBus` over tokio broadcast, one channel per event
  type created lazily; `publish` fans a `ControlEvent` (type + JSON payload +
  `CorrelationId`, contract #3) out to every `subscribe`r of its type and to no
  other type — a no-subscriber publish is a zero-reach no-op, not a failure.
  `livequery` module: `subscribe_table` opens a SurrealDB `LIVE SELECT` on a
  WS-03 gate-issued scoped session's connection, so SurrealDB row-level
  permissions decide which records the subscriber sees — scope set once at
  subscribe, not proxied per message (contract #1); the `DataChangeStream` maps
  each notification's `Action` to a `DataChange` (Created/Updated/Deleted),
  decoding the record through a new `rubix_core::decode_record` (reuses the one
  record decode path, no row-shape duplication). A `Killed` query ends the
  stream; an `Action::Error` surfaces as `BusError::Evaluation`. Verified on
  kv-mem: an insert in a principal's namespace pushes a `Created` event; a
  foreign-namespace write is never delivered to that principal's subscriber.

- **WS-09 — DataFusion query surface over SurrealDB + vector search.** New
  `rubix-query` crate, the unification/compute layer (SCOPE "DataFusion — query
  and compute"). DataFusion sits **above** SurrealDB only for unification and
  heavy aggregation (contract #6); SurrealQL stays first. `provider` module:
  each canonical table (`record`/`tag`/`audit`/`insight`) is scanned through the
  principal's WS-03 scoped session via plain SurrealQL `SELECT`, projected into a
  fixed Arrow schema (structural columns + free-form `content` JSON string,
  principle 4) and registered as an in-memory table — so the only rows DataFusion
  can plan over are the ones SurrealDB row-level permissions already admitted
  (contract #1), and there is no unscoped base table to escape; a not-yet-written
  table (`insight` ahead of WS-11) scans as empty. `query` module: `run`
  executes a single `SELECT`/`WITH` over that context after a statement `guard`
  rejects any write/DDL/multi-statement input (the read-only injection guard,
  parsed with DataFusion's own parser); `run_authorized` gates the query action
  on the WS-04 `external-query` capability before any scan, fail closed (contract
  #2). `aggregate` module: `rollup_window` reads a numeric `content.<field>`
  series through the scoped session and folds it into epoch-aligned buckets
  (`minute…week`) with `avg/min/max/sum/count/first/last` per bucket — the
  vectorized window values that feed a Rhai rule decision (WS-11). `search`
  module: `nearest` runs SurrealDB-native `vector::distance::euclidean` k-nearest
  over a record's vector column on the scoped session (SurrealQL first), with the
  table/field-path validated as identifiers to close the one injection surface.

- **WS-10 — Datasource connector framework + Postgres connector.** New
  `rubix-datasource` crate, the pluggable-connector layer above WS-09's unified
  surface (SCOPE "Datasources": a datasource is a declared, pluggable connection
  each registering as a DataFusion `TableProvider`; adding a connector, not
  changing the core). `connector` module: a `Connector` trait (declared
  `DatasourceConfig` in → async `TableProvider` out) and the config identity the
  registry keys on. `registry` module: a `Registry` keyed by datasource id, seeded
  with the native SurrealDB default (`with_native_default`); `register` runs the
  pipeline `authorize` (WS-04 `datasource-register` capability, fail closed) →
  duplicate/reserved-native-id guard → materialise each connector table's provider
  once → insert; `resolve` looks a datasource id up (unknown id fails closed, not
  silently empty); `list` returns the declared identities for dashboards; `span`
  is how the unified surface reads the registry — it gates the query action on the
  WS-04 `external-query` capability, builds the scoped SurrealDB context through
  `rubix_query::build_context` (so contract #1 still bounds the SurrealDB rows),
  registers each external connector's providers under a per-id catalog schema
  (addressed `"<id>"."<table>"`, no collision with the native canonical names),
  then runs one read-only `SELECT`/`WITH` across both. `surreal` module: the
  native engine exposed through the same `Connector` contract (`SurrealConnector`),
  reusing the WS-09 scoped scan rather than re-reading rows. `postgres` module
  (`#[cfg(feature = "postgres")]`): a `PostgresConnector` over
  `datafusion-table-providers` (DataFusion 53), connecting a pooled client from a
  libpq string and materialising declared tables as providers — absent the feature
  it never compiles in, so the connector fails closed on the default edge build.
  Verified on kv-mem: a granted principal registers a second (SurrealDB-backed)
  connector and a single query spans the native and registered sources; an
  ungranted register and an ungranted query are both denied; a query naming an
  unregistered datasource fails to plan; the Postgres round-trip skips cleanly when
  `RUBIX_TEST_PG` is unset. Minimal additive change to WS-09: `build_context`
  widened to `pub` so the scoped scan is reused, not duplicated.

- **WS-11 — Rhai rules / insights runtime.** New `rubix-rules` crate, the
  embedded deterministic rule/insight runtime (SCOPE "Rhai — rules and
  insights"; `rubix/STACK-DEISGN.md`, "Rhai owns the decision; DataFusion owns
  the data"). `engine` module: a hardened Rhai `Engine` (bounded operations and
  call depth so a script fails deterministically rather than hanging) with one
  host fn `invoke(id)` resolving a sub-rule's already-computed decision value,
  and a `Decision` (fired/value/reason) normalised from a script's bool or
  decision-map return — a non-decision return is an error, no guessed fallback.
  `rule` module: a `Rule` (script + input `Binding`s + declared `subrules` +
  output kind), a `Binding` that resolves a script input to a DataFusion window
  value (an `Aggregate` of the latest `rollup_window` bucket pulled through the
  principal's scoped session, contract #1 — a series with no bucket fails the
  binding, never a silent zero), and a `RuleRegistry` that resolves sub-rules
  fail-closed. `evaluate` module: `evaluate` mints one correlation id, evaluates
  the rule and its sub-rules depth-first against the window values (`compose`
  + `run`), emits and persists a WS-08 span per node (`span`, two-phase span ids
  so the tree nests without a mutable span), records the root decision as an
  insight through the WS-05 gate as a `rule-invoke` command (`record` — a fresh
  generic record, authorized + captured + correlated + audited, fail closed when
  ungranted), and publishes the firing on the WS-07 in-process bus (`publish`)
  under the same correlation id. The one id threads the decision, insight, audit
  row, event, and every span (contract #3), so `rubix_trace::assemble_trace`
  retrieves the full per-evaluation "why". Verified on kv-mem: a rule fires
  offline on a window value and records an insight + audit row; an ungranted
  evaluation is denied with no insight written; a composed rule (rule-calls-rule)
  reflects its child and nests the sub-rule span under the parent; the assembled
  span tree shows each rule, the values seen, and the decision. Additive bus
  surface: the `insight.recorded` control-event type for live insight firings.

- **WS-12 — Zenoh ingestion + pre-processing.** New `rubix-ingest` crate, the
  streaming-ingestion path (SCOPE "Ingestion and pre-processing": sources publish
  to Zenoh; the platform consumes in flight and persists before, not after,
  query). `subscribe` module: `authorize_keyspace` is the **single capability
  decision taken at subscribe** (contract #2) — it checks the WS-04
  `zenoh-subscribe` grant fail closed, then confirms the requested key expression
  is *included* in the principal's edge-identity key-space subtree
  (`rubix/ingest/<namespace>/**`), and only then yields an `AuthorizedKeySpace`,
  the sole way `open_subscription` gets a scope; an out-of-grant or out-of-partition
  key-space is refused once, before any Zenoh session opens, so a high-rate stream
  is never re-taxed per message. `IngestSubscriber` opens a Zenoh peer session
  (`ZenohEndpoint` → `zenoh::Config`), declares the subscriber on the resolved
  scope, and decodes each sample payload into a free-form-JSON `Sample`. `process`
  module: the in-flight pre-processing nodes — `Decimator` (stateful 1-in-N rate
  cut, factor clamped ≥1), `Filter` (predicate drop), `Enricher` (merge derived
  fields, the one node that rewrites) — composed in order by `Pipeline`
  (decimate → filter → enrich, each optional, a drop short-circuits the rest).
  `persist` module: `append_sample` writes each survivor as a fresh-id `Record`
  into the partition keyed by the principal's namespace (the edge identity,
  contract #5) — append-only, never updating an existing row, and *not* re-crossing
  the command gate per sample (the capability was decided once at subscribe);
  `partition_for`/`keyspace_root` are the one place the edge partition is derived,
  so the subscribe scope and the persistence partition cannot drift. Verified on
  kv-mem + a local Zenoh peer pair over TCP loopback: a granted principal opens a
  subscription on its edge key-space, published samples flow through
  decimate+filter, and exactly the survivors land append-only under the edge
  partition; an ungranted subscribe and an out-of-partition key-space are both
  refused at subscribe. Additive workspace change: the `zenoh` dependency
  (`default-features = false`, `transport_tcp`).

- **WS-16 — Transport: axum HTTP + WS live-query bridge + OpenAPI + prefs.**
  New `rubix-prefs` crate (the Preferences component, SCOPE "Preferences"):
  per-user **units** (metric↔imperial over tagged physical quantities —
  temperature/length/mass/speed, conversion math owned by one `Quantity` type so
  the round-trip cannot drift) and **datetime** formatting (strftime pattern over
  a canonical RFC 3339 UTC instant), applied to a response DTO by `apply_to`,
  which rewrites only the declared fields and leaves the rest untouched — storage
  stays canonical, only the display DTO is transformed. `rubix-server` now wires
  the committed crates into the axum binary: record CRUD routes where mutations
  cross the WS-05 gate (capability-checked, captured atomically, correlated,
  audited — an audit row per write) and reads run on the WS-03 scoped session
  (contract #1); `POST /query` gating the DataFusion surface on the WS-04
  `external-query` capability and rendering Arrow batches to JSON rows; `GET
  /datasources` listing the registry; a **WebSocket bridge** (`/ws/records`)
  streaming the WS-07 live-query data-change feed to a client on its scoped
  session (scope set once at subscribe, contract #1); and **OpenAPI 3.1** via
  utoipa at `/api-docs/openapi.json`. Principal authentication reuses the gate's
  subject/secret `PrincipalToken` over `x-rubix-subject`/`x-rubix-secret` headers,
  resolved through `authenticate` + `issue_scoped_session` once per request. A
  record mutation requires the `IngestPublish` capability (the committed gate enum
  has no dedicated record-write variant; the single point of choice is
  `http/records/capability.rs`). Verified on kv-mem: `/health`; a create→get→
  update→delete round-trip over HTTP with an audit row per mutation carrying the
  correlation id; a WebSocket client receiving the `created` event on insert; the
  OpenAPI document listing every registered route; and a record DTO formatted per
  the user's unit/datetime preference. Deferred (require WS-13/14, logged to
  `docs/sessions/TODOs.md`): the JSON-RPC extension control endpoint and `POST
  /datasources` registration (need `rubix-ext`), and profile selection into
  `AppState` (WS-14). Additive workspace deps: `chrono`, `utoipa`.

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
- [x] Capability grants — app-enforced authz for cross-plane (non-record) actions.
- [x] Command gate — every mutation through the gate; `RETURN BEFORE` capture.
- [x] Audit log — append-only, immutable, correlation-id stamped.
- [x] Undo/redo — reversible change records for definitions, applied through gate.

### Event bus
- [x] In-process tokio control plane.
- [x] Data-change plane over SurrealDB live queries, permission-filtered.
- [x] Tracing spans on the bus, bounded/sampled retention.

### Query / compute
- [x] DataFusion `TableProvider` over SurrealDB + unified `/query` surface.
- [x] Vectorized time-window aggregation feeding rule decisions.
- [x] Vector / semantic search surface.
- [x] Datasource connector framework + Postgres connector.
- [ ] MQTT / REST connectors (follow-up behind the framework).

### Rules / insights
- [x] Rhai embedded runtime — composable rules, offline, decision → SurrealDB +
      data-change event, span tree per evaluation.

### Ingestion / transport
- [x] Zenoh ingestion + pre-processing (decimate/filter/enrich), edge-partitioned.
- [x] Key-space scope resolved once at subscribe (capability decision).

### Extensions
- [ ] Extension principal model + JSON-RPC control plane + Zenoh data plane.

### Platform / deployment
- [ ] Edge/cloud profiles (single binary, cargo features + runtime config).
- [ ] Edge↔cloud sync shipper over Zenoh (append-only partition + config LWW).
- [x] Preferences (units + datetime).
- [x] Transport: axum HTTP + WS live-query bridge + OpenAPI (`rubix-server`).
      JSON-RPC extension control + `POST /datasources` registration deferred with
      WS-13 (extensions); profile selection deferred with WS-14 — see
      `docs/sessions/TODOs.md` 2026-06-15T09:50:00Z.

---

## Environment variables

| Var | Default | Used by | Meaning |
| --- | --- | --- | --- |
| `RUBIX_NAMESPACE` | `rubix` | `rubix-server` | SurrealDB namespace to bootstrap/use. |
| `RUBIX_DATABASE` | `main` | `rubix-server` | SurrealDB database to bootstrap/use. |
| `RUBIX_DATA_DIR` | `rubix-data` | `rubix-server` | SurrealKV file-backed data directory. |
| `RUBIX_BIND` | `127.0.0.1:8080` | `rubix-server` | HTTP listen address. |
| `RUBIX_TRACE_SAMPLE` | `0.0` | `rubix-trace` | Span drop fraction `[0.0, 1.0]`; `0.0` keeps all, `1.0` drops all. |
| `RUBIX_TEST_PG` | _(unset)_ | `rubix-datasource` tests | Postgres URL enabling the feature-gated connector round-trip test; unset skips it cleanly. |

---

## Build & test

```
cd rubix
cargo test --workspace
cargo clippy --workspace --all-targets
```
