# Rubix Testing & Feedback Suite

> **You are an AI session picking this up cold. Read this file top to bottom, then
> jump to the doc that matches your task.** Every doc is a self-contained runbook:
> exact commands, exact files, exact pass/fail checks. Nothing here assumes prior
> context beyond what is written down.

Rubix is an edge-to-cloud data platform backend: a standalone Cargo workspace
(`rubix/`, *not* part of the awaken workspace). It models records with a schemaless
JSON model, runs a scoped read/write gate, evaluates rules via a Rhai sandboxed
layer, queries via DataFusion over SurrealDB, manages multi-level capabilities,
and ships data over Zenoh for edge/cloud sync.

This suite does three things:

1. **Unit + integration tests green** — each WS-xx (workstream) ships comprehensive
   tests: `cargo test --workspace` + `cargo clippy --workspace --all-targets` are
   required gates.
2. **Live-stack verification** — once the HTTP transport (WS-16) lands, stand up
   the server, hit the API, and exercise end-to-end flows.
3. **Close a feedback loop** — capture consistent evidence (logs, git state, test
   output) so an AI session can diagnose and fix rubix, then re-run and confirm.

---

## The architecture (read this once)

```
┌─────────────────────────────────────────────────────────────┐
│ rubix-server (axum binary)                                  │
│  ├─ main.rs (boot: store + gate + bus + rules + query)     │
│  ├─ http/ (routes: records, tags, query, rules, datasrc)   │
│  ├─ ws/ (live-query WebSocket bridge)                      │
│  └─ rpc/ (JSON-RPC for extensions — WS-13)                │
├─────────────────────────────────────────────────────────────┤
│ rubix-gate (scoped read + capability authz)                │
│  ├─ principal (identity model)                             │
│  ├─ command (write gate + audit)                           │
│  └─ capability (fail-closed grants)                        │
├─────────────────────────────────────────────────────────────┤
│ rubix-store (SurrealDB embedded)                           │
│  ├─ read/write on scoped sessions                          │
│  └─ schema (record, tag, audit, grant tables)              │
├─────────────────────────────────────────────────────────────┤
│ rubix-query (DataFusion over SurrealDB)                    │
│  ├─ window rollups + vector search                         │
│  ├─ pluggable datasources (native + Postgres)              │
│  └─ unified SQL surface                                    │
├─────────────────────────────────────────────────────────────┤
│ rubix-rules (Rhai sandboxed rules engine)                  │
│  ├─ curated vectorized primitives                          │
│  ├─ no-row-explosion guarantee                             │
│  └─ org-scoped rule storage                                │
├─────────────────────────────────────────────────────────────┤
│ rubix-ingest (Zenoh pub/sub + pre-processing)             │
│  └─ in-flight record transforms                            │
├─────────────────────────────────────────────────────────────┤
│ rubix-bus (event plane: in-process + live-query)          │
│  ├─ control events (ControlEvent)                          │
│  └─ data-change stream (subscription filter)               │
└─────────────────────────────────────────────────────────────┘
```

Key facts that shape testing:

- **No auth by default** — edge profile omits OIDC. Cloud requires it (WS-14).
- **Scoped by namespace + row-level perms** — SurrealDB enforces read scope via
  `$auth.namespace` (WS-03); writes cross the command gate (WS-05).
- **Capabilities are fail-closed** — every operation checks a grant; absent or
  ungranted capability = denied (WS-04).
- **Zenoh is the ingestion plane** — edge publishes, cloud subscribes, conflict
  model is append-only data + LWW config (WS-15, deferred).
- **SurrealDB is the store** — embedded on edge (`rubix.db`), Postgres federation
  behind a feature gate (WS-14, deferred).

---

## Map of this suite

| Path | What it is | When to read it |
|------|-----------|-----------------|
| [`BUILD.md`](BUILD.md) | Cargo build: unit tests, integration tests, clippy | Start here for local dev. |
| [`QUICKSTART.md`](QUICKSTART.md) | Boot the server, ping health, first API call | Once WS-16 (transport) lands. |
| [`00_setup/STACK.md`](00_setup/STACK.md) | Every process, port, env var, tear-down | When something won't start. |
| [`features/`](features/) | One runbook per major feature | When testing/fixing that feature. |
| [`scenarios/`](scenarios/) | Cross-feature golden-path scripts | End-to-end / regression. |
| [`feedback-loop/CAPTURE.md`](feedback-loop/CAPTURE.md) | Evidence bundle for diagnosis | When something breaks. |
| [`reference/ARCHITECTURE.md`](reference/ARCHITECTURE.md) | Grounded system map, file:line citations | When a doc's claim looks stale. |

---

## How to work in this suite (the operating contract)

1. **Run unit + integration tests**: `cargo test --workspace`.
2. **Check code quality**: `cargo clippy --workspace --all-targets`.
3. **If a test fails**, capture evidence via feedback-loop runbook, diagnose, and fix.
4. **Once WS-16 lands**, bring up the stack, run feature runbooks, and close the
   live-stack feedback loop.
5. **If a doc's fact is wrong**, fix the doc, re-run affected tests, and note the
   fix in the feature doc.

Each feature doc carries a `> Verified:` line. Anything older than the current
`main` is unverified — re-test before trusting it.

---

## Conventions used throughout

- Commands assume CWD `rubix/`.
- Use `make test` for the test suite (`cargo test --workspace`).
- Use `make lint` for clippy (`cargo clippy --workspace --all-targets`).
- `$BASE` = `http://127.0.0.1:8088` (the `make` default for the server).
- `rubix.db` = the embedded SurrealDB file (reset by deleting it).
- Evidence lands in `testing/.evidence/<scenario>/<timestamp>/` (git-ignored).
- ✅ / ❌ checkboxes in runbooks are literal pass/fail gates.

---

## Status at a glance

| Phase | Workstream | Status | Tests | Clippy | Notes |
|-------|------------|--------|-------|--------|-------|
| Core | WS-01 | ✅ | Green | Clean | Workspace foundation + SurrealDB |
| Core | WS-02 | ✅ | Green | Clean | Record model + tag graph |
| Core | WS-03 | ✅ | Green | Clean | Identity + scoped session |
| Core | WS-04 | ✅ | Green | Clean | Capability grants (authz layer 2) |
| Core | WS-05 | ✅ | Green | Clean | Command gate + audit |
| Core | WS-06 | ✅ | Green | Clean | Undo/redo |
| Core | WS-07 | ✅ | Green | Clean | Event bus (control + live-query) |
| Core | WS-08 | ✅ | Green | Clean | Tracing spans |
| Core | WS-09 | ✅ | Green | Clean | DataFusion query surface |
| Core | WS-10 | ✅ | Green | Clean | Datasource connectors |
| Core | WS-11 | ✅ | Green | Clean | Rhai rules/insights runtime |
| Core | WS-12 | ✅ | Green | Clean | Zenoh ingestion + pre-processing |
| WIP | WS-16 | 🔵 | TBD | TBD | Transport: HTTP + JSON-RPC + WS bridge |
| Deferred | WS-13 | ⛔ | — | — | Extensions (deferred) |
| Deferred | WS-14 | ⛔ | — | — | Profiles (deferred) |
| Deferred | WS-15 | ⛔ | — | — | Sync shipper (deferred) |

---

## Quick checklist: is it ready to test?

- [ ] `make lint` passes (no warnings)
- [ ] `make test` passes (all tests green)
- [ ] No untracked files in critical paths (`crates/`, `ui/`)
- [ ] `git status` is clean
- [ ] Latest `main` is pulled

If all pass, pick a feature doc and run the steps.
