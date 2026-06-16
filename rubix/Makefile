# Rubix — backend (Rust/cargo) + UI (pnpm/vite) developer tasks.
#
#   make build       build backend + UI
#   make dev         run backend + UI together (Ctrl-C stops both)
#   make dev SEED=1  ...seeding the demo portfolio if the store is empty
#   make seed        wipe the store and boot a freshly-seeded dev stack
#   make dev-be      run just the backend
#   make dev-ui      run just the UI
#   make test        run backend + UI tests
#   make lint        clippy (backend) + eslint (UI)
#   make fmt         cargo fmt + prettier
#   make build-be    build just the backend
#   make build-ui    build just the UI
#   make clean       remove build artifacts (cargo target + UI dist)
#   make wipe        delete the dev store (rubix-data) but KEEP the cargo target
#   make kill        free the dev ports if a previous run was left running
#   make db-up       start a local TimescaleDB (Postgres) container
#   make db-down     stop + remove the container (keeps the data volume)
#   make db-clean    stop + remove the container and delete its data volume
#   make docs        run the docs site dev server (Nextra)
#   make docs-build  static-export the docs site to doc-site/out
#   make docs-clean  remove the docs build output + generated changelog
#
# The backend is the cargo workspace in this directory; the UI is a pnpm/vite
# app under ui/.

UI_DIR := ui

# Docs site (Nextra). A pnpm workspace package (`@rubix/doc-site`) that lives in
# doc-site/ and statically exports to doc-site/out. DOCS_PORT pins the dev
# server (package.json defaults to 3010; keep in sync or override both). The
# docs build is independent of the cargo backend — it needs only pnpm.
DOCS_DIR := doc-site
DOCS_PKG := @rubix/doc-site
DOCS_PORT ?= 3010

# Dev ports — kept in sync with the code. rubix-server binds RUBIX_BIND
# (default 127.0.0.1:8080 in crates/rubix-server/src/main.rs); the UI's vite
# dev server listens on UI_PORT and proxies the API to VITE_API_PROXY (defaults
# in ui/vite.config.ts mirror the values below). We point the backend at BE_PORT
# and pin VITE_API_PROXY/UI_PORT to it so the proxy reaches the backend out of
# the box. 8092/5192 avoid the common 8088/5180 clashes. Override to remap.
BE_PORT ?= 8092
UI_PORT ?= 5192
# The binary reads RUBIX_BIND (crates/rubix-server/src/main.rs); keep this name
# in sync or the backend silently falls back to its default port and the UI proxy
# can't reach it.
export RUBIX_BIND ?= 127.0.0.1:$(BE_PORT)
# Pin the UI proxy + UI port to the Makefile's values so the dev server always
# follows the backend — overriding BE_PORT/UI_PORT alone is enough and the UI
# can never silently proxy to a stale port. vite.config.ts reads both.
export VITE_API_PROXY ?= http://127.0.0.1:$(BE_PORT)
export VITE_UI_PORT ?= $(UI_PORT)

# Backend store directory. The server reads RUBIX_DATA_DIR (defaulting to
# `rubix-data`, see crates/rubix-server/src/main.rs); pin it here so the seed
# guard below can test whether the store already exists, and so `make kill` and
# `make seed` know which dir holds the lock. The backend runs with cwd = this
# directory, so a relative path lands right here next to the Makefile.
export RUBIX_DATA_DIR ?= rubix-data

# Backend cargo feature profile. Empty = the default `edge` build (SQLite,
# on-box drivers, no auth). Set FEATURES=cloud to compile the Postgres store
# backend + DataFusion Postgres federation surface, then point RUBIX_DB at a
# postgres:// url and RUBIX_PROFILE=cloud at boot. Example:
#   make dev-be FEATURES=cloud RUBIX_PROFILE=cloud RUBIX_DB=postgres://localhost/rubix
FEATURES ?=
CARGO_FEATURES := $(if $(FEATURES),--features $(FEATURES),)

# Dev portfolio seed. Off by default: a fresh DB starts empty. Set SEED=1 to
# populate the demo "acme"/"globex" portfolio (sites + equips/points/readings +
# rules). Handy for UI work. Example:
#   make dev-be SEED=1
#
# `--seed-dev` is NOT idempotent: seed_dev() re-provisions principals + records on
# every boot and dies on a non-fresh store ("already exists" / duplicate ids — see
# crates/rubix-server/src/seed/mod.rs). So we append it ONLY when SEED=1 AND the
# store dir doesn't exist yet (a store that's already there is already seeded).
# This makes `make dev SEED=1` safe to run repeatedly: first run seeds, later runs
# just boot. To force a fresh re-seed, run `make seed` (it wipes the store first).
SEED ?=
STORE_EXISTS := $(wildcard $(RUBIX_DATA_DIR))
SEED_DEV := $(if $(SEED),$(if $(STORE_EXISTS),,--seed-dev),)
# Program args after cargo's `--`. We ALWAYS pass the `--rubix` marker: main.rs
# only scans argv for `--seed-dev`/`--migrate-history` and ignores anything else,
# but the marker tags this server on its command line so `make kill` can pkill
# ONLY this project's backend and never the sibling nhp backend (which runs the
# same binary from this same workspace, tagged `--nhp`).
RUN_ARGS := -- --rubix $(SEED_DEV)

# Embedded agent. Off by default (the server boots with RUBIX_AI=0). Set AI=1 to
# turn on /agent/chat, spark dispatch, and the agent status surface. The model
# provider reads its key from the environment at run time, so export the key in
# your shell first — never put it in this file or any committed file. Example:
#   export OPENAI_API_KEY=sk-...   # in your shell, not here
#   make dev AI=1
# Override provider/model/gate with the usual RUBIX_AI_* vars on the CLI.
AI ?=
ifeq ($(AI),1)
export RUBIX_AI := 1
endif

# Local TimescaleDB (Postgres) for the rubix-datasource Postgres connector.
# Dockerfile lives next to the connector code. DB_PORT maps the container's
# 5432 onto the host; override to avoid clashing with a native Postgres. With
# the defaults the connection string is:
#   postgres://rubix:rubix@127.0.0.1:$(DB_PORT)/rubix
DB_DIR := crates/rubix-datasource/docker
DB_IMAGE := rubix-timescaledb
DB_CONTAINER := rubix-timescaledb
DB_VOLUME := rubix-timescale-data
DB_PORT ?= 5433

.PHONY: build dev dev-be dev-ui seed test test-be test-ui lint fmt \
        build-be build-ui clean wipe kill db-up db-down db-clean \
        docs docs-build docs-clean

build: build-be build-ui

build-be:
	cargo build $(CARGO_FEATURES)

build-ui:
	cd $(UI_DIR) && pnpm install && pnpm build

# Run both in one foreground process group; Ctrl-C (or `make kill`) stops both.
# The trap reaps the children on exit so no orphan server keeps a port held.
dev:
	@echo "backend → http://$(RUBIX_BIND)   ui → http://127.0.0.1:$(UI_PORT)"
	@trap 'kill 0' EXIT INT TERM; \
	( cargo run --bin rubix-server $(CARGO_FEATURES) $(RUN_ARGS) ) & \
	( cd $(UI_DIR) && pnpm dev --port $(UI_PORT) ) & \
	wait

dev-be:
	cargo run --bin rubix-server $(CARGO_FEATURES) $(RUN_ARGS)

# Wipe the store and boot a freshly-seeded dev stack. rubix has no separate seed
# step (the demo portfolio is written at boot behind `--seed-dev`, not over HTTP),
# so "seed" means: clear the store, then `make dev` with the seed forced on. Use
# this when you want a guaranteed-clean demo portfolio; for an ordinary boot that
# only seeds an empty store, use `make dev SEED=1`.
seed:
	rm -rf $(RUBIX_DATA_DIR)
	$(MAKE) dev SEED=1

dev-ui:
	cd $(UI_DIR) && pnpm dev

test: test-be test-ui

test-be:
	cargo test $(CARGO_FEATURES)

test-ui:
	cd $(UI_DIR) && pnpm test

lint:
	cargo clippy $(CARGO_FEATURES) --all-targets -- -D warnings
	cd $(UI_DIR) && pnpm lint

fmt:
	cargo fmt
	cd $(UI_DIR) && pnpm format

clean:
	cargo clean
	rm -rf $(UI_DIR)/dist
	rm -rf $(RUBIX_DATA_DIR)

# Wipe ONLY the dev store ($(RUBIX_DATA_DIR)) — the database, not the build. Use
# this to reset to an empty store (e.g. after a SIGKILL wedged the WAL so the
# backend hangs at boot) or to reclaim disk, WITHOUT paying for the full
# `cargo clean` + recompile that `make clean` forces. Stops any running server
# first (via `kill`) so nothing holds the store's LOCK as the dir is removed; a
# following `make dev` then boots a fresh, empty store. To wipe AND re-seed the
# demo portfolio in one step, use `make seed` instead.
wipe: kill
	rm -rf $(RUBIX_DATA_DIR)
	@echo "wiped store $(RUBIX_DATA_DIR) (cargo target kept)"

# Free the dev ports AND kill any orphaned backend/UI left behind by a crashed
# run. The port sweep alone is not enough: when `make dev` segfaults its trap
# never fires, the children reparent to init, and an orphaned rubix-server keeps
# holding `$(RUBIX_DATA_DIR)/LOCK` (and contends on the shared cargo target lock)
# even after it loses its port — so the next boot blocks on the lock, not the
# port. fuser is also absent on some boxes, so we also drive the kill off pkill.
#
# pkill -f matches THIS project's signatures only: the UI's `vite --port
# $(UI_PORT)` and the backend's `--rubix` marker (see RUN_ARGS). Targeting the
# marker — not the bare binary name — is what keeps this from killing the sibling
# nhp backend, which runs the same binary from this same workspace (tagged
# `--nhp`). The '-' prefixes keep make from failing when a tool is missing or
# nothing matches.
#
# The pkill patterns use a bracket class on the first char (`[v]ite`,
# `[r]ubix-server`) so they don't match THEMSELVES. make expands the recipe into
# the literal command line of the shell that runs pkill, so a plain `vite.*`
# pattern matched that very shell (its argv contains the pattern text) and pkill
# SIGKILL'd its own parent — the `Killed (ignored)` noise that sometimes aborted
# the recipe before it reached the backend kill. `[v]ite` is a regex that still
# matches the real `vite` process, but the pattern *string* is `[v]ite`, which
# has no `vite` substring, so pkill's own command line no longer self-matches.
#
# We signal SIGTERM first, NOT SIGKILL — and that distinction is load-bearing.
# rubix-server now traps SIGTERM and shuts the store down cleanly (see
# `shutdown_signal` in crates/rubix-server/src/main.rs). The file-backed
# SurrealKV store appends every commit to a write-ahead log; a SIGKILL landing
# mid-append leaves a torn record at the WAL tail that wedges replay on the next
# boot — the store opens but hangs forever, so the server never binds 8092 and
# the UI proxy spins on ECONNREFUSED. (That is exactly the failure this target
# used to CAUSE: it SIGKILL'd the backend, which over repeated runs corrupted
# rubix-data and made `make dev` appear to "keep crashing".) SIGTERM lets the
# backend finish in-flight writes and exit with a whole WAL.
#
# After signalling, we poll with pgrep until the processes actually exit. We poll
# pgrep, not fuser: fuser/ss/lsof are absent on this box (the fuser lines above
# silently no-op here), so the process table is the only reliable signal. The
# poll doubles as the graceful-shutdown grace period: it waits up to ~8s for a
# clean exit, then escalates the stragglers to SIGKILL as a last resort (a wedged
# or hung process must not block the port forever). The wait also closes the
# race where a returning `make kill` followed immediately by `make dev` raced the
# dying process for the port/LOCK.
#
# The backend pattern is `rubix-server.* --rubix` (not `rubix-server --rubix`)
# so it matches BOTH the running binary (`target/debug/rubix-server --rubix`)
# AND the cargo wrapper still compiling it (`cargo run --bin rubix-server --
# --rubix`, where the `--` separator sits between the two tokens). Killing only
# the bare binary missed the compile window: a `make kill` fired mid-compile hit
# nothing, cargo then exec'd an orphaned server holding the port/LOCK.
kill:
	-@fuser -TERM -k $(BE_PORT)/tcp 2>/dev/null || true
	-@fuser -TERM -k $(UI_PORT)/tcp 2>/dev/null || true
	-@pkill -TERM -f '[v]ite.* --port $(UI_PORT)' 2>/dev/null || true
	-@pkill -TERM -f '[r]ubix-server.* --rubix' 2>/dev/null || true
	@i=0; \
	while pgrep -f '[r]ubix-server.* --rubix' >/dev/null 2>&1 \
	   || pgrep -f '[v]ite.* --port $(UI_PORT)' >/dev/null 2>&1; do \
		i=$$((i+1)); \
		if [ $$i -ge 80 ]; then \
			pkill -KILL -f '[v]ite.* --port $(UI_PORT)' 2>/dev/null || true; \
			pkill -KILL -f '[r]ubix-server.* --rubix' 2>/dev/null || true; \
			break; \
		fi; \
		sleep 0.1; \
	done
	@echo "freed ports $(BE_PORT)/$(UI_PORT) and killed any orphaned backend/UI"

# Docs site dev server (Nextra) at http://127.0.0.1:$(DOCS_PORT). The package's
# `predev` regenerates the Changelog page from CHANGELOG.md before starting.
# `pnpm --filter` runs from the workspace root regardless of cwd, so this works
# whether or not the dir argument matches; we pass it for clarity.
docs:
	@echo "docs → http://127.0.0.1:$(DOCS_PORT)"
	pnpm --filter $(DOCS_PKG) install
	pnpm --filter $(DOCS_PKG) dev --port $(DOCS_PORT)

# Static-export the docs to $(DOCS_DIR)/out (no server at runtime). Set
# DOCS_BASE_PATH/DOCS_REPO_URL in the environment for a sub-path or repo-linked
# deploy; both default to empty/neutral. `postbuild` writes out/.nojekyll.
docs-build:
	pnpm --filter $(DOCS_PKG) install
	pnpm --filter $(DOCS_PKG) build

# Remove the docs build output and the generated (gitignored) changelog page.
# Leaves node_modules in place — use a plain `pnpm` reinstall to reset those.
docs-clean:
	rm -rf $(DOCS_DIR)/out $(DOCS_DIR)/.next $(DOCS_DIR)/content/changelog.mdx
	@echo "cleaned docs build output ($(DOCS_DIR)/out, .next, generated changelog)"

# Build the image from the Dockerfile, run it detached on a named volume, then
# poll pg_isready until it accepts queries — so a follow-on `make dev` finds a
# live backend. Removes any stale container of the same name first.
db-up:
	docker build -t $(DB_IMAGE) $(DB_DIR)
	-@docker rm -f $(DB_CONTAINER) 2>/dev/null || true
	docker run -d --name $(DB_CONTAINER) \
		-p $(DB_PORT):5432 \
		-v $(DB_VOLUME):/var/lib/postgresql/data \
		$(DB_IMAGE)
	@echo "waiting for timescaledb to accept queries..."
	@for i in $$(seq 1 30); do \
		docker exec $(DB_CONTAINER) pg_isready -U rubix -d rubix >/dev/null 2>&1 && break; \
		sleep 1; \
	done
	@echo "timescaledb → postgres://rubix:rubix@127.0.0.1:$(DB_PORT)/rubix"

# Stop and remove the container; the named data volume survives so the next
# `make db-up` keeps existing data.
db-down:
	-@docker rm -f $(DB_CONTAINER) 2>/dev/null || true
	@echo "removed container $(DB_CONTAINER) (volume $(DB_VOLUME) preserved)"

# Stop and remove the container AND delete its data volume — a clean slate.
db-clean:
	-@docker rm -f $(DB_CONTAINER) 2>/dev/null || true
	-@docker volume rm $(DB_VOLUME) 2>/dev/null || true
	@echo "removed container $(DB_CONTAINER) and volume $(DB_VOLUME)"
