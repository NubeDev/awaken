# Stack — Every Process, Port, Env Var, Teardown

> Verified: code-grounded on `rubix-gaps` tip, 2026-06-13. Source of truth for env
> vars is `crates/rubix-server/src/main.rs`. Re-grep `RUBIX_` there if a default
> below looks stale.

---

## Processes

| Component | Binary / dir | `make` endpoint | Notes |
|-----------|--------------|-----------------|-------|
| API + bus + supervisor | bin `rubix` (crate `rubix-server`) | `127.0.0.1:8088` | single process; opens SQLite, zenoh, spawns drivers. Bare-binary default is `0.0.0.0:8080`; `make` sets `RUBIX_ADDR=127.0.0.1:8088` |
| UI (optional) | `ui/` (pnpm/vite) | `127.0.0.1:5180` | dev server; proxies `/api` to `VITE_API_PROXY` (default `http://127.0.0.1:8088`) |
| Reference driver | `rubix-driver-sim` (`crates/rubix-driver-sim`) | — | spawned by the supervisor, not run by hand |
| Store | SQLite file | `rubix.db` | WAL; no DB server. Postgres is a `cloud`-feature option |
| Data plane | zenoh | peer mode | a single node needs no router; multi-node can use one |

There is **no broker, no Postgres** required for the edge path. The API is one
binary plus the SQLite file plus zenoh peering; the UI is optional.

## Makefile targets

The `Makefile` wraps cargo + pnpm and keeps ports in sync (`BE_PORT ?= 8088`,
`UI_PORT ?= 5180`; override on the CLI to remap):

| Target | Does |
|--------|------|
| `make build` / `build-be` / `build-ui` | build both / backend (`cargo build`) / UI (`pnpm build`) |
| `make dev` | run backend **and** UI in one process group (Ctrl-C stops both) |
| `make dev-be` | run just the backend (`cargo run --bin rubix`, binds `:8088`) |
| `make dev-ui` | run just the UI (`pnpm dev`, `:5180`) |
| `make test` / `test-be` / `test-ui` | `cargo test` / `pnpm test` |
| `make lint` | `cargo clippy --all-targets -- -D warnings` + `pnpm lint` |
| `make fmt` | `cargo fmt` + `pnpm format` |
| `make kill` | free `:8088` / `:5180` if a previous run was left bound |
| `make clean` | `cargo clean` + remove `ui/dist` |

**Cargo features:** `make <target> FEATURES=cloud` compiles the Postgres store
backend + DataFusion Postgres federation. Pair it with `RUBIX_PROFILE=cloud` and a
`postgres://` `RUBIX_DB` at boot, e.g.
`make dev-be FEATURES=cloud RUBIX_PROFILE=cloud RUBIX_DB=postgres://localhost/rubix`.

---

## Environment variables

Read via `env_or(...)` / `std::env` in `crates/rubix-server/src/main.rs`. Defaults
in parentheses.

| Env var | Purpose | Default |
|---------|---------|---------|
| `RUBIX_PROFILE` | Deployment profile (`edge`/`cloud`) | compiled default (edge if feature on) |
| `RUBIX_DB` | SQLite file path, or `postgres://…` URL (cloud) | `rubix.db` |
| `RUBIX_ADDR` | Server listen `addr:port` | `0.0.0.0:8080` (binary); `make` exports `127.0.0.1:8088` |
| `RUBIX_ZENOH` | Enable zenoh data plane + supervisor + sub-driven writes | `1` |
| `RUBIX_DRIVERS` | Path to driver-manifests JSON | `drivers.json` |
| `RUBIX_QUERY` | Enable DataFusion `/query` surface | `1` |
| `RUBIX_SCHEDULER` | Enable board scheduler (interval + `cur`-sub triggers) | `1` |
| `RUBIX_HIS_PARQUET` | Parquet cold-tier root (enables `/his/flush`) | unset (SQLite-only his) |
| `RUBIX_AI` | Enable embedded awaken agent + `/agent/chat` | `0` (**off**) |
| `RUBIX_AI_DISPATCH` | Enable spark→agent dispatch (needs bus **and** agent) | `1` |
| `RUBIX_AI_MIN_PRIORITY` | Agent write ceiling (1..=16) — commits at/below | `13` |
| `RUBIX_AI_ESCALATION_FLOOR` | Lowest slot reachable with approval (1..=ceiling) | `1` |
| `RUBIX_AI_PROVIDER` | LLM provider name (genai) | `openai` |
| `RUBIX_AI_MODEL_ID` | Local model id | `gpt-4o-mini` |
| `RUBIX_AI_MODEL` | Upstream model name | value of `RUBIX_AI_MODEL_ID` |
| `RUBIX_AI_MAX_ROUNDS` | Max agent tool-call rounds | `8` |
| `RUBIX_OIDC_ISSUER` | OIDC JWT issuer URL (enables auth on edge) | unset |
| `RUBIX_OIDC_JWKS` | OIDC JWKS endpoint URL | unset |

Notes:
- The LLM API key is read **at run time** by the genai provider, not at boot — a
  node with `RUBIX_AI=1` boots fine without a key and only errors when a model
  call is attempted.
- **Cloud profile requires** `RUBIX_OIDC_ISSUER` + `RUBIX_OIDC_JWKS` and fails
  boot if they're missing. Edge leaves auth off unless they're set.

---

## Subsystem on/off matrix

Each subsystem is independent; turning one off does not break the HTTP API.

| Disable with | Effect |
|--------------|--------|
| `RUBIX_ZENOH=0` | no `cur` pub/sub, no write/his queryables, **no supervisor → no sim driver**, subscription boards skipped with a warning |
| `RUBIX_QUERY=0` | `POST /api/v1/query` returns `503` |
| `RUBIX_AI=0` (default) | `POST /api/v1/agent/chat` returns `503`; no dispatch |
| `RUBIX_SCHEDULER=0` | stored boards never auto-fire (still runnable by slug) |
| unset `RUBIX_HIS_PARQUET` | `POST /api/v1/his/flush` returns `503`; his is SQLite-only |

---

## Minimal vs full bring-up

- **Minimal (API + store only):** `RUBIX_ZENOH=0 make dev-be` — CRUD, query, boards
  over the store; no live data, no drivers.
- **Live data (default):** `make dev-be` with a `drivers.json` — the QUICKSTART path.
- **With agent:** `RUBIX_AI=1 RUBIX_AI_PROVIDER=… <key env> make dev-be` — enables
  `/agent/chat`, dispatch, and the `agent_call` board node.
- **With UI:** `make dev` — backend `:8088` + UI `:5180` together.

---

## Teardown

```bash
# Ctrl-C the server: graceful shutdown stops dispatcher → scheduler → supervisor
# in order, reaping spawned drivers so liveliness tokens clear.
make kill                                  # free :8088 / :5180 if a run was left bound
rm -f rubix.db rubix.db-wal rubix.db-shm   # clean store baseline
rm -f drivers.json                          # if you want no drivers next boot
```

A clean baseline is just: no `rubix.db*` files and a `drivers.json` you control.
There are no containers or background services to reap — `make kill` only frees the
dev ports.
