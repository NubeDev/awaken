# NHP — the rubix backend (Rust/cargo, built next door) + the NHP UI (pnpm/vite).
#
#   make build       build backend + UI
#   make dev         run backend + UI together (Ctrl-C stops both)
#   make dev SEED=1  ...and populate the demo portfolio (off by default)
#   make dev-be      run just the backend (rubix-server)
#   make dev-ui      run just the NHP UI
#   make test        run UI tests
#   make lint        clippy (backend) + eslint (UI)
#   make fmt         cargo fmt + prettier
#   make build-be    build just the backend
#   make build-ui    build just the UI
#   make clean       remove UI build artifacts
#   make kill        free the dev ports if a previous run was left running
#
# NHP has no backend crate of its own — it is a thin domain + UI layer on the
# already-built rubix backend (see ../docs/OVERVIEW.md). RUBIX_DIR points at the
# rubix cargo workspace; all cargo commands run there. The UI is the pnpm/vite
# app under ui/.
UI_DIR := ui
RUBIX_DIR := ../rubix

# Dev ports. NHP picks 8094/5194 so it can run alongside rubix (8092/5192) and
# the common 8088/5180 defaults without a clash (see ../docs/sessions/WS-01.md).
# rubix-server binds RUBIX_BIND; the UI's vite dev server listens on UI_PORT and
# proxies the rubix routes to VITE_API_PROXY (vite.config.ts reads both env vars).
BE_PORT ?= 8094
UI_PORT ?= 5194
# rubix-server reads RUBIX_BIND (rubix crates/rubix-server/src/main.rs); keep this
# in sync or the backend falls back to its default port and the UI proxy misses.
export RUBIX_BIND ?= 127.0.0.1:$(BE_PORT)
# Pin the UI proxy + UI port to this Makefile's values so the dev server always
# follows the backend — overriding BE_PORT/UI_PORT alone is enough.
export VITE_API_PROXY ?= http://127.0.0.1:$(BE_PORT)
export VITE_UI_PORT ?= $(UI_PORT)

# Backend store directory. CRITICAL: NHP runs the backend out of the rubix
# workspace (RUBIX_DIR) but MUST NOT share rubix's own `rubix-data/` SQLite store
# — both opening the same file-backed store fights over `rubix-data/LOCK` and the
# second boot dies with "Database at rubix-data/LOCK is already locked". The
# server reads RUBIX_DATA_DIR (rubix crates/rubix-server/src/main.rs); point it at
# an NHP-private dir, resolved ABSOLUTELY so it lands here next to this Makefile
# (the backend runs with cwd = RUBIX_DIR, so a relative path would land in rubix/).
# Gitignored; `make clean` removes it. Override to relocate.
export RUBIX_DATA_DIR ?= $(abspath nhp-data)

# Backend cargo feature profile (passed through to the rubix workspace). Empty =
# the default edge build (SQLite, no auth). FEATURES=cloud compiles the Postgres
# store + multi-tenant surface; point RUBIX_DB/RUBIX_PROFILE at it as rubix docs.
FEATURES ?=
CARGO_FEATURES := $(if $(FEATURES),--features $(FEATURES),)

# Dev portfolio seed. Off by default: a fresh DB starts empty.
#
# SEED=1 turns on rubix's own --seed-dev, which is a PREREQUISITE for the NHP seed,
# not the NHP portfolio itself: it creates the `acme`/`globex` namespaces and the
# operator/admin/agent principals the NHP seed authenticates as (there is no HTTP
# path to provision identities — they precede any scoped session). The NHP mock
# portfolio + faked poller data is a SEPARATE step (`make seed`) that writes data
# over the HTTP records API against the already-running, --seed-dev'd backend.
#
# Required order (WS-03):
#   1. make dev SEED=1     # boot rubix --seed-dev (namespace + principals) + UI
#   2. make seed           # (in another shell) register collections + portfolio
#   3. make seed-check     # confirm the expected counts
# It repopulates on a re-run but is idempotent-ish (records carry stable keys and
# are skipped if present); deleting the DB clears it until the next seeded run.
SEED ?=
SEED_DIR := seed

# Program args passed to rubix-server (after cargo's `--`). We ALWAYS pass the
# `--nhp` marker: the server ignores unknown args (main.rs only scans argv for
# `--seed-dev`), but the marker tags the NHP backend on its command line so
# `make kill` can pkill ONLY this server and never the sibling rubix project's
# backend (which has no port/data-dir on its cmdline to grep — both are env-only).
# `--seed-dev` is appended when SEED=1.
RUN_ARGS := -- --nhp $(if $(SEED),--seed-dev,)

# Where the collection registrar + check scripts POST. Defaults follow the dev
# backend; override RUBIX_BASE / RUBIX_SUBJECT / RUBIX_SECRET (collections/client.mjs)
# to target another server. The defaults match a `--seed-dev` server in namespace
# `acme` (the seeded `acme_operator` holds the record-write capability).
COLLECTIONS_DIR := collections
export RUBIX_BASE ?= http://127.0.0.1:$(BE_PORT)

.PHONY: build dev dev-be dev-ui test lint fmt build-be build-ui clean kill collections collections-check seed seed-nhp seed-check smoke

build: build-be build-ui

build-be:
	cd $(RUBIX_DIR) && cargo build $(CARGO_FEATURES)

build-ui:
	cd $(UI_DIR) && pnpm install && pnpm build

# Vite file-watch mode. On a shared box where `fs.inotify.max_user_watches` is
# exhausted by other processes, vite crashes on boot with `ENOSPC: file watchers
# reached` (it watches every source file via inotify). POLL=1 sets VITE_POLL, so
# vite.config.ts switches to polling — no inotify watches at all (a little idle
# CPU, HMR still works). Use `make dev POLL=1` when you hit ENOSPC.
POLL ?=
ifeq ($(POLL),1)
export VITE_POLL := 1
endif

# Run both in one foreground process group; Ctrl-C (or `make kill`) stops both.
# The trap reaps the children on exit so no orphan server holds a port.
dev:
	@echo "backend → http://$(RUBIX_BIND)   ui → http://127.0.0.1:$(UI_PORT)$(if $(POLL),   (vite polling),)"
	@trap 'kill 0' EXIT INT TERM; \
	( cd $(RUBIX_DIR) && cargo run --bin rubix-server $(CARGO_FEATURES) $(RUN_ARGS) ) & \
	( cd $(UI_DIR) && pnpm dev --port $(UI_PORT) ) & \
	wait

dev-be:
	cd $(RUBIX_DIR) && cargo run --bin rubix-server $(CARGO_FEATURES) $(RUN_ARGS)

dev-ui:
	cd $(UI_DIR) && pnpm dev --port $(UI_PORT)

test:
	cd $(UI_DIR) && pnpm test

# Register the 7 NHP collection definitions (kind:"collection" records) on a
# RUNNING rubix backend, idempotently. NHP has no backend crate — these are data
# created over the rubix HTTP API (see ../docs/sessions/WS-02.md). Needs node and
# a backend reachable at RUBIX_BASE. Run `make dev SEED=1` first.
collections:
	node $(COLLECTIONS_DIR)/register-collections.mjs

# Verify the collections end-to-end: one valid + one invalid record per kind
# (gate-enforced required/type; client-enforced enum/unique/device-limit).
collections-check:
	node $(COLLECTIONS_DIR)/check.mjs

# Seed the NHP mock portfolio + faked poller data on a RUNNING --seed-dev'd rubix
# backend (WS-03). Registers the collections first, then writes
# tenant→site→gateway→network→meter→register records (stamped from the meter-types,
# tagged for dashboard auto-build) plus history for history=true registers and
# faked status/last_seen. Idempotent-ish; needs node + a reachable RUBIX_BASE.
# Run `make dev SEED=1` first (it provisions the namespace + principals). `seed-nhp`
# is an alias matching SEED.md's naming.
seed seed-nhp:
	node $(SEED_DIR)/seed.mjs

# Confirm the seeded portfolio meets the WS-03 done-gate counts (2 tenants,
# 4 sites, a full register set, meters tagged, history present).
seed-check:
	node $(SEED_DIR)/check.mjs

# End-to-end smoke (WS-08): boot a THROWAWAY rubix --seed-dev backend (data dir in
# rubix/, gitignored, removed on exit) → register collections → seed → seed-check →
# records-check (the dashboard data pipeline returns rows) → pnpm build →
# pnpm test:unit. One entrypoint wiring the existing checks; gates the POC. Needs
# nothing running first — it owns its own backend. See scripts/smoke.sh.
smoke:
	bash scripts/smoke.sh

lint:
	cd $(RUBIX_DIR) && cargo clippy $(CARGO_FEATURES) --all-targets -- -D warnings
	cd $(UI_DIR) && pnpm lint

fmt:
	cd $(RUBIX_DIR) && cargo fmt
	cd $(UI_DIR) && pnpm format

clean:
	rm -rf $(UI_DIR)/dist
	rm -rf $(RUBIX_DATA_DIR)

# Free the dev ports AND kill any orphaned NHP backend/UI left behind by a crashed
# run. The port sweep alone is not enough: when `make dev` segfaults its trap never
# fires, the children reparent to init, and an orphaned rubix-server keeps holding
# `$(RUBIX_DATA_DIR)/LOCK` even after it loses its port — so the next boot dies on
# the lock, not the port (the "LOCK is already locked" error). fuser is also absent
# on some boxes (then the old port-only kill silently did nothing), so we drive the
# kill off pkill, which is always present.
#
# pkill -f matches NHP-private signatures only: the UI's `--port $(UI_PORT)` (which
# tags the make-dev wrapper, pnpm, and vite together) and the backend's `--nhp`
# marker arg (see RUN_ARGS). Targeting the marker — not the bare binary name — is
# what keeps this from killing the sibling rubix project's own rubix-server, which
# runs the same binary from the same workspace. The '-' prefixes keep make from
# failing when a tool is missing or nothing matches.
kill:
	-@fuser -k -KILL $(BE_PORT)/tcp 2>/dev/null || true
	-@fuser -k -KILL $(UI_PORT)/tcp 2>/dev/null || true
	-@pkill -KILL -f -- '--port $(UI_PORT)' 2>/dev/null || true
	-@pkill -KILL -f 'rubix-server --nhp' 2>/dev/null || true
	@echo "freed ports $(BE_PORT)/$(UI_PORT) and killed any orphaned NHP backend/UI"
