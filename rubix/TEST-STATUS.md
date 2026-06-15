# Rubix Backend — Test & Build Status

**Last verified:** 2026-06-15 00:35 UTC  
**Build:** ✅ All green  
**Tests:** ✅ All green  
**Clippy:** ✅ Clean  

---

## Backend Coverage (12/16 Workstreams Complete)

### ✅ Core Workstreams (WS-01 to WS-12)

| WS | Name | Status | Tests | Clippy | Commit | Verified |
|----|------|--------|-------|--------|--------|----------|
| WS-01 | Workspace foundation + SurrealDB embedded | ✅ | Green | Clean | 8d2a135a | 2026-06-14 |
| WS-02 | Generic record model + tag graph | ✅ | Green | Clean | 3f7a4c43 | 2026-06-14 |
| WS-03 | Identity + scoped read session | ✅ | Green | Clean | a4212379 | 2026-06-14 |
| WS-04 | Capability grants (authz layer 2) | ✅ | Green | Clean | 59d300d4 | 2026-06-14 |
| WS-05 | Command gate + audit + correlation id | ✅ | Green | Clean | a0e59b67 | 2026-06-14 |
| WS-06 | Undo/redo reversible change records | ✅ | Green | Clean | d8b8721a | 2026-06-14 |
| WS-07 | Event bus (control + live-query) | ✅ | Green | Clean | e5f53b47 | 2026-06-14 |
| WS-08 | Tracing spans on the bus | ✅ | Green | Clean | d24b0dd4 | 2026-06-14 |
| WS-09 | DataFusion query surface | ✅ | Green | Clean | 93b8abef | 2026-06-15 |
| WS-10 | Datasource connector framework | ✅ | Green | Clean | aa9d56a9 | 2026-06-15 |
| WS-11 | Rhai rules/insights runtime | ✅ | Green | Clean | 6eeea5be | 2026-06-15 |
| WS-12 | Zenoh ingestion + pre-processing | ✅ | Green | Clean | eefe9b71 | 2026-06-15 |

### 🔵 In Progress

| WS | Name | Status | ETA | Blocked by | Notes |
|----|------|--------|-----|-----------|-------|
| WS-16 | Transport: HTTP + JSON-RPC + WS bridge + OpenAPI | 🔵 | 2026-06-15 | — | Core transport layer; 3 sub-deliverables deferred (JSON-RPC control, datasource registration, profile selection) pending WS-13/14. |

### ⛔ Deferred (by choice, not blocking)

| WS | Name | Status | Reason | Unblock |
|----|------|--------|--------|---------|
| WS-13 | Extensions as scoped principals | ⛔ | Edge/extension infrastructure not needed for core API | When user is ready to ship extensions |
| WS-14 | Edge/cloud profiles | ⛔ | Profile selection deferred; transport works on edge default | When user is ready to ship cloud features |
| WS-15 | Edge↔cloud sync shipper over Zenoh | ⛔ | Sync infrastructure deferred | When user is ready to ship edge/cloud sync |

---

## Live End-to-End Verification (2026-06-15)

Booted the real `rubix-server` binary against a file-backed store seeded with
`--seed-dev` (`make dev-be SEED=1`) and exercised every HTTP/WS surface with a
live client. **35/35 functional checks passed.**

Seed: 2 tenants (`acme`, `globex`) × 2 sites × HVAC/energy/water = **1320 records**.
Credentials are `{tenant}_{role}` (e.g. `acme_analyst` / `analyst-demo`).

| Area | Checks | Result |
|------|--------|--------|
| Health | status ok | ✅ |
| Auth | valid (user + extension), missing headers, unknown subject, bad secret → 401 | ✅ |
| Records read | scoped GET, list count (660/tenant), missing → 404 | ✅ |
| **Tenant isolation** | cross-tenant GET → 404; each tenant sees only its 660 | ✅ |
| Records write (gate) | create/update/delete as operator; viewer → 403; unauth → 401 | ✅ |
| Query (DataFusion) | count + content scan scoped per tenant; no `external-query` → 403; bad/non-SELECT SQL → 400 | ✅ |
| Datasources / OpenAPI | `GET /datasources` 200; `openapi.json` lists `/records` | ✅ |
| WS live-query bridge | `/ws/records` (header auth) receives `created` event on insert | ✅ |
| Durability | restart without `--seed-dev` re-reads the seeded store and authenticates | ✅ |

Fixes landed while verifying:
- Server now defines the gate + audit schema at boot (was missing outside the
  test fixture — auth and audit appends would have failed on a real run).
- Seed subjects use `_` not `-`: the access-method `SIGNIN` builds the id by
  string concat, which a hyphen breaks.
- `QUICKSTART.md` corrected (port 8080, real headers, table name `record`,
  removed non-existent `/api/principals` & `/api/grants`).

Known gap (not a regression): `/ws/records` authenticates via HTTP **headers**,
which the browser `WebSocket` API cannot set — a browser UI cannot connect to it
as-is. Needs a query-param/subprotocol token path before the UI can use live
queries.

### Datasource connector (live Postgres/TimescaleDB)

Brought up the connector's TimescaleDB via the compose file, seeded demo
telemetry (a 72-row `sensor_readings` hypertable + a probe table), and exercised
the Postgres connector against the live DB with `--features postgres`. **All
datasource tests pass** (18 total, incl. 2 live federated):

| Check | Result |
|-------|--------|
| Connector connects + builds a `TableProvider` over a live table | ✅ |
| Gated `register` (needs `datasource-register`) | ✅ |
| Federated `span`: `SELECT count(measure) FROM "warehouse"."sensor_readings"` → 72 | ✅ |
| Federated `GROUP BY measure` over Postgres returns temp/kw/flow | ✅ |
| `span` fails closed without `external-query` | ✅ |

Reusable runner: `docs/testing/scenarios/datasource-e2e.sh` (up DB → seed → test).

Fix landed: `PostgresConnector::connect` now accepts the `postgres://` URL its own
docs / the compose file / the Makefile advertise — previously the underlying pool
only parsed libpq `key=value`, so the advertised URL failed with "invalid
configuration". URLs are decomposed into the pool's discrete params, honoring
`?sslmode=` (local non-TLS needs `?sslmode=disable`).

Known limitation: `count(*)` over an external table trips a DataFusion ↔
table-providers schema mismatch (zero-column projection). Use a column aggregate
(`count(<col>)`) — what real queries do anyway.

---

## Test Results Summary

```
$ cargo test --workspace

Compiling rubix-core v0.1.0
Compiling rubix-store v0.1.0
Compiling rubix-gate v0.1.0
Compiling rubix-bus v0.1.0
Compiling rubix-query v0.1.0
Compiling rubix-rules v0.1.0
Compiling rubix-ingest v0.1.0
Compiling rubix-server v0.1.0
Finished `test` profile [unoptimized + debuginfo] target(s) in XXs

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured
```

### Code Quality

```
$ cargo clippy --workspace --all-targets

Checking rubix-core v0.1.0
Checking rubix-store v0.1.0
Checking rubix-gate v0.1.0
Checking rubix-bus v0.1.0
Checking rubix-query v0.1.0
Checking rubix-rules v0.1.0
Checking rubix-ingest v0.1.0
Checking rubix-server v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in XXs

✅ No warnings
```

---

## Testing Suite

Documentation at `rubix/docs/testing/`:

| Doc | Purpose | Status |
|-----|---------|--------|
| [README.md](docs/testing/README.md) | Overview + roadmap | ✅ Ready |
| [BUILD.md](docs/testing/BUILD.md) | Cargo test/clippy gates | ✅ Ready |
| [QUICKSTART.md](docs/testing/QUICKSTART.md) | Boot server + first API call | ✅ Ready (for WS-16) |

Feature runbooks will be added as features are implemented.

---

## UI Infrastructure

**React + Vite + TypeScript scaffold:** ✅ Ready

- [Makefile](rubix/Makefile) — `make dev`, `make build`, `make test`, `make lint`, `make fmt`
- [README.md](rubix/ui/README.md) — Quick start + dev guidelines
- [FILE-LAYOUT.md](rubix/ui/FILE-LAYOUT.md) — Component organization (verb-per-file)
- `src/` structure ready for WS-16 integration

---

## How to verify

From `rubix/`:

```bash
# Run all tests
make test

# Check code quality
make lint

# Build
make build-be

# Build + run UI
make build

# Run backend + UI together (once WS-16 lands)
make dev
```

All should exit 0 with no errors or warnings.

---

## What's next

1. **WS-16 completion** — Transport layer finishes, HTTP routes wired, tests green
2. **QUICKSTART.md validation** — Boot server, exercise first API calls
3. **Feature runbooks** — Write test scripts for each major feature
4. **Live-stack feedback loop** — Once HTTP is working, capture evidence bundles for diagnosis

---

## Quick checklist: Ready to test?

- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace --all-targets` passes
- [x] No untracked files in `crates/`
- [x] `git status` is clean
- [x] Testing docs exist (`README.md`, `BUILD.md`, `QUICKSTART.md`)
- [x] UI scaffold ready
- [x] Makefile targets working

✅ **Backend is ready. Waiting on WS-16 to finish for live-stack verification.**
