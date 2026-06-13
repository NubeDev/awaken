# Rubix Architecture — Grounded Map for Testing

> Verified: code-grounded on `rubix-gaps` tip, 2026-06-13. **Re-grep before
> trusting any file:line below** — citations rot. If a claim has drifted, fix it
> here first, bump this line, then continue. Authoritative narrative status lives
> in `rubix/STATUS.md`; this is the testing-facing extract.

This is the factual substrate the rest of the testing docs stand on. Deliberately
terse and citation-heavy: when a runbook step surprises you, come here to confirm
how the system *actually* behaves.

---

## 1. Processes, ports, storage

| Component | Binary | Default endpoint | Notes |
|-----------|--------|------------------|-------|
| API + bus + supervisor | bin `rubix` (crate `rubix-server`) | `0.0.0.0:8080` binary / `127.0.0.1:8088` via `make` | `RUBIX_ADDR` overrides (`main.rs:64`); `[[bin]] name="rubix"` |
| Store | SQLite file | `RUBIX_DB` (`rubix.db`) | WAL, r2d2 pool; Postgres under cloud feature |
| Data plane | zenoh peer session | — | `RUBIX_ZENOH=0` disables (`main.rs:83`) |
| Reference driver | `rubix-driver-sim` | — | spawned by supervisor from `RUBIX_DRIVERS` |

Boot sequence (`crates/rubix-server/src/main.rs:32-268`): tracing → profile select
→ open store → open zenoh + `serve()` + launch supervisor → his tier (opt-in) →
query engine → auth config → agent runtime (opt-in) → scheduler → spark dispatcher
→ bind `RUBIX_ADDR` and serve with graceful shutdown.

---

## 2. Domain model (`rubix-core`)

`crates/rubix-core/src/model.rs`:
- **Site** (`:8`): `org`, `slug`, `display_name`, `tags`. Identity prefix `{org}/{slug}`.
- **Equip** (`:19`): `site_id`, `path` (slash-nested, e.g. `ahu-3/fan`), `tags`.
- **Point** (`:49`): `equip_id`, `slug`, `kind: PointKind` (`Sensor`/`Cmd`/`Sp`),
  `unit`, `priority_array`, `cur_value`, `cur_ts`. Keyexpr = `{org}/{site}/{equip-path}/{point}`
  (`Point::keyexpr`, `:67`). Sensor is read-only; Cmd/Sp carry a priority array.
- **Spark** (`:90`): rule finding — `rule`, `severity` (`Info`/`Warning`/`Fault`),
  `message`, `point_ids`, `acknowledged`.
- **PointValue** (`value.rs:1`): untagged `Bool|Number|Str`; wire form `{"value": 21.5}`.

### 16-level priority array (`rubix-core/src/priority.rs`)

- `PRIORITY_LEVELS = 16` (`:6`). 16 slots + a `relinquish_default`.
- `set(priority, value)` (`:43`) writes slot `1..=16`, rejects out-of-range.
- `relinquish(priority)` (`:51`) clears a slot, returns prior value.
- `effective()` (`:59`) — **lowest level number wins**; falls back to
  `relinquish_default`. Tests at `:78-126` (`lower_level_number_wins`,
  `relinquish_returns_previous_value`, `rejects_out_of_range_priority`).

### Tags / slugs (`rubix-core/src/tags.rs`)

`TagSet` = `BTreeMap<String, Value>` (markers map to `true`). `has_all` matches on
presence. `validate_slug` enforces lowercase `[a-z0-9-]` for every path segment.

---

## 3. Zenoh data plane (`rubix-server/src/bus`)

- Opens a peer session in `main` (`RUBIX_ZENOH=1`).
- **Publishes** `cur` on `{org}/{site}/{equip}/{point}/cur` on every ingest, write,
  relinquish (including bus-driven writes).
- **Subscribes** to `cur` from drivers and lands the value in `points.cur_value` +
  appends history.
- **Serves queryables** `**/write` (priority-array command) and `**/his/**`
  (history) **scoped to owned sites**: a node answers only for keys under a
  `{org}/{site}` it holds (reusing `Capability::covers`) and stays silent
  otherwise — no "not found" noise in a mesh. Sites added after boot are covered
  without re-declaring.
- **Publishes sparks** on `{org}/{site}/spark/{rule}/{id}` on `POST /sparks` and on
  board `emit_spark`, so cloud subscribers (alerting, agent dispatch) see findings
  live.

---

## 4. Query surface (`rubix-query`)

- `QueryEngine` registers canonical tables `sites`/`equips`/`points`/`his`/`sparks`
  as read-only DataFusion `TableProvider`s over SQLite (`context/tables.rs:8`).
  Schema read live from `PRAGMA table_info`, so empty tables still resolve columns.
- **`points_cur` view** (`context/register.rs:70`) flattens each point's effective
  `cur_value`/`cur_ts` and joins site/equip to expose the resolved keyexpr:
  `SELECT keyexpr, cur_value FROM points_cur`.
- `POST /api/v1/query` (gated `RUBIX_QUERY`) runs one read-only `SELECT`/`WITH`.
- `POST /api/v1/his/rollup` — time-bucketed aggregates. Intervals: `minute`,
  `five_minute`, `fifteen_minute`, `hour`, `day`, `week` (`rollup/spec.rs:26`).
  Aggregates: `avg`/`min`/`max`/`sum`/`count`/`first`/`last` (`:50`). Only point
  ids are interpolated (validated, quoted); everything else is a closed enum.
- **Tenant scope** (`context/scope.rs`): a `QueryScope` is one `{org}/{site}` pair;
  scoped queries read through tenant-filtered views. Rejects values with quotes/NUL.
- SQLite-only except `his`, which can union a Parquet cold tier when attached.
  Postgres federation exists under the cloud feature (`open_postgres`).

---

## 5. Reflow boards (`rubix-flow`)

- `BoardGraph` JSON = `nodes` + `connections` (`board/schema.rs:12`). A node is
  `{id, component, config}`; a connection is `{from_node, from_port, to_node, to_port}`.
- **Component palette** (`board/registry.rs:14` `COMPONENTS`, 6 built-ins):
  `read_point`, `write_point`, `query_his`, `emit_spark`, `agent_call`, `trigger`
  (a self-paced timing source — `src/node/trigger/`).
- `BoardGraph::run(access)` (`board/run.rs:43`): single-shot — start network, tick
  source nodes (no inbound connection), settle (`SETTLE=50ms`, `MAX_SETTLE=120s`),
  collect every outport. Writes always go through the priority array.
- Nodes depend on the `PointAccess` trait (`port.rs:59`): `read_point`,
  `write_point(key, priority, value)`, `query_his`, `emit_spark`, `request_agent`
  (detached) / `request_agent_blocking` (awaited).
- **Stored + versioned**: `boards` table keyed `(slug, version)`; `POST
  /api/v1/boards` republishes, `POST /api/v1/boards/{slug}/run` runs latest.
- **Scheduler** (`rubix-server/scheduler`): one task per scheduled board;
  `Interval { seconds }`, `Subscription { key }` (fires on each matching `cur`),
  or `Manual`. Re-reads its board each trigger, so republish/disable takes effect
  next tick. `RUBIX_SCHEDULER=0` disables; subscription boards need the bus.

---

## 6. AI tools + agent (`rubix-tools` + `rubix-server`)

- Four `TypedTool`s over `PointAccess`: `read_point` (read-only), `write_point`
  (priority-gated), `query` (read-only `SELECT`/`WITH` only — rejects multi-statement),
  `run_board` (board over the store). Plus `pin_widget`.
- **Three-band write gating** (`rubix-tools/src/tool/write_point.rs`): at/below the
  agent ceiling (`priority >= agent_min_priority`) **commits**; the escalation band
  (`escalation_floor <= priority < ceiling`) **suspends** with a `SuspendTicket`,
  store untouched; below the floor is **denied**. `DEFAULT_PRIORITY = 16`.
- **Tenant scope** (`rubix-tools/src/scope.rs`): a run carries `{org}/{site}`; tools
  refuse any keyexpr outside it (`covers`, path-boundary), and the SQL `query` tool
  runs through a tenant-filtered session.
- `POST /api/v1/agent/chat` (gated `RUBIX_AI`, default off) runs a tool-calling turn.
  An escalation-band write returns `status: awaiting_approval` + a `run_id`.
- **Runs surface**: chat/dispatch/MCP runs persist to a `runs` table. `GET
  /api/v1/runs`, `GET /api/v1/runs/{id}`, `POST .../resume` (re-applies the held
  write, gating re-checked, one-shot), `POST .../cancel`.
- **Spark dispatch** (`dispatch` module, `RUBIX_AI_DISPATCH`, needs bus + agent):
  subscribes `**/spark/**`, activates an agent *run* (job, not chat) per finding on
  a spark-keyed thread. Closes the board → spark → bus → agent loop.
- **MCP** (`mcp/mod.rs`): `POST /api/v1/mcp` speaks JSON-RPC 2.0
  (`initialize`/`tools/list`/`tools/call`) into the *same* scoped tool registry —
  identical priority gating, scope, and HITL escalation as the embedded agent.

---

## 7. Driver contract + supervisor (`rubix-driver` + `rubix-server/supervisor`)

- `DriverManifest` = identity + `point_types` + `CapabilitySet` + opaque `config`
  (`manifest/mod.rs:19`). `validate()` rejects empty identity and a no-capability
  manifest.
- `Capability { prefix, access }` (`manifest/capability.rs:35`). `access`:
  `Publish`/`Subscribe`/`All`. **`covers(key)`** (`:65`) — `key == prefix` or sits
  beneath it on a `/` boundary. This *same* function is the tenancy primitive for
  bus owned-site scoping, query scope, and agent tool scope.
- **Supervisor** (`rubix-server/supervisor`): loads manifests from `RUBIX_DRIVERS`
  (`manifests.rs`; missing file → no drivers, malformed → fail closed), spawns one
  child per manifest with env-injected identity/caps/config, tracks liveliness
  tokens (`await_attach`/`await_clear`), jittered exponential backoff on crash,
  shared shutdown. Live spawn covered by `rubix-driver-sim/tests/supervised.rs`.

---

## 8. Auth (optional)

- OIDC-JWT + PAT via bearer middleware. **Off by default on edge**; cloud requires
  `RUBIX_OIDC_ISSUER` + `RUBIX_OIDC_JWKS` (fails boot if absent).
- `Principal { subject, scope, role }`; `Scope` is the `{org}/{team}/{site}`
  hierarchy; `Role` is `Operator`/`Service`/`Viewer`. `may_write` needs a writing
  role and scope coverage.
- Public paths bypass auth always: `/healthz`, `/api-docs/openapi.json`.
- PATs minted via `POST /api/v1/tokens`, scoped to the issuer's org/site, cannot
  escalate privilege.

---

## 9. Existing test coverage (where to look before writing new tests)

| Crate | Location | What |
|-------|----------|------|
| rubix-core | `src/priority.rs`, `src/tags.rs` (inline) | priority semantics, tag/slug validation |
| rubix-query | `tests/query.rs`, `tests/rollup.rs`, `tests/his_tier.rs`, `tests/scoped.rs` | SQL over SQLite, rollups, scope |
| rubix-flow | `tests/board.rs` | load, run, awaited agent_call, JSON round-trip |
| rubix-tools | `tests/read_point.rs`, `tests/write_point.rs`, `tests/query.rs` | tool exec, priority bands, read-only guard |
| rubix-driver | `src/manifest/{capability,mod}.rs` (inline) | covers logic, access direction, manifest validation |
| rubix-driver-sim | `tests/supervised.rs`, `src/{simulate,scoped}.rs` | live spawn/attach/publish/shutdown, scope denial |

`cargo test` runs all of it (125 tests across 6 crates as of 2026-06-12 review).
There are **no HTTP-level integration tests for the server's route handlers** yet —
that gap is what the feature runbooks here exercise by hand.
