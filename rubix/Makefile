# Rubix — backend (Rust/cargo) + UI (pnpm/vite) developer tasks.
#
#   make build       build backend + UI
#   make dev         run backend + UI together (Ctrl-C stops both)
#   make dev SEED=1  ...and populate the demo portfolio (off by default)
#   make dev-be      run just the backend
#   make dev-ui      run just the UI
#   make test        run backend + UI tests
#   make lint        clippy (backend) + eslint (UI)
#   make fmt         cargo fmt + prettier
#   make build-be    build just the backend
#   make build-ui    build just the UI
#   make clean       remove build artifacts (cargo target + UI dist)
#   make kill        free the dev ports if a previous run was left running
#
# The backend is the cargo workspace in this directory; the UI is a pnpm/vite
# app under ui/.

UI_DIR := ui

# Dev ports — kept in sync with the code. rubix-server binds RUBIX_ADDR
# (default 0.0.0.0:8080 in crates/rubix-server/src/main.rs); the UI's vite
# dev server listens on 5180 and proxies /api to VITE_API_PROXY (default
# http://127.0.0.1:8088 in ui/vite.config.ts). We point the backend at :8088
# below so the UI proxy reaches it out of the box. Override on the CLI to remap.
BE_PORT ?= 8088
UI_PORT ?= 5180
export RUBIX_ADDR ?= 127.0.0.1:$(BE_PORT)

# Backend cargo feature profile. Empty = the default `edge` build (SQLite,
# on-box drivers, no auth). Set FEATURES=cloud to compile the Postgres store
# backend + DataFusion Postgres federation surface, then point RUBIX_DB at a
# postgres:// url and RUBIX_PROFILE=cloud at boot. Example:
#   make dev-be FEATURES=cloud RUBIX_PROFILE=cloud RUBIX_DB=postgres://localhost/rubix
FEATURES ?=
CARGO_FEATURES := $(if $(FEATURES),--features $(FEATURES),)

# Dev portfolio seed. Off by default: a fresh DB starts empty. Set SEED=1 to
# populate the demo "acme" portfolio (4 sites + equips/points/history) and run
# the live-value ticker — handy for UI work, but it repopulates on every boot,
# so deleting the DB only clears it until the next seeded run. Example:
#   make dev-be SEED=1
SEED ?=
SEED_FLAG := $(if $(SEED),-- --seed-dev,)

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

.PHONY: build dev dev-be dev-ui test test-be test-ui lint fmt \
        build-be build-ui clean kill

build: build-be build-ui

build-be:
	cargo build $(CARGO_FEATURES)

build-ui:
	cd $(UI_DIR) && pnpm install && pnpm build

# Run both in one foreground process group; Ctrl-C (or `make kill`) stops both.
# The trap reaps the children on exit so no orphan server keeps a port held.
dev:
	@echo "backend → http://$(RUBIX_ADDR)   ui → http://127.0.0.1:$(UI_PORT)"
	@trap 'kill 0' EXIT INT TERM; \
	( cargo run --bin rubix $(CARGO_FEATURES) $(SEED_FLAG) ) & \
	( cd $(UI_DIR) && pnpm dev ) & \
	wait

dev-be:
	cargo run --bin rubix $(CARGO_FEATURES) $(SEED_FLAG)

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

# Free the dev ports if a previous run was killed ungracefully and left a
# listener behind. Best-effort: fuser exits non-zero when nothing is bound, so
# the leading '-' keeps make from failing.
kill:
	-@fuser -k -KILL $(BE_PORT)/tcp 2>/dev/null || true
	-@fuser -k -KILL $(UI_PORT)/tcp 2>/dev/null || true
	@echo "freed ports $(BE_PORT) and $(UI_PORT)"
