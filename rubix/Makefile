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
#   make db-up       start a local TimescaleDB (Postgres) container
#   make db-down     stop + remove the container (keeps the data volume)
#   make db-clean    stop + remove the container and delete its data volume
#
# The backend is the cargo workspace in this directory; the UI is a pnpm/vite
# app under ui/.

UI_DIR := ui

# Dev ports — kept in sync with the code. rubix-server binds RUBIX_BIND
# (default 127.0.0.1:8080 in crates/rubix-server/src/main.rs); the UI's vite
# dev server listens on 5180 and proxies the API to VITE_API_PROXY (default
# http://127.0.0.1:8088 in ui/vite.config.ts). We point the backend at :8088
# below so the UI proxy reaches it out of the box. Override on the CLI to remap.
BE_PORT ?= 8088
UI_PORT ?= 5180
# The binary reads RUBIX_BIND (crates/rubix-server/src/main.rs); keep this name
# in sync or the backend silently falls back to its default port and the UI proxy
# can't reach it.
export RUBIX_BIND ?= 127.0.0.1:$(BE_PORT)

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

.PHONY: build dev dev-be dev-ui test test-be test-ui lint fmt \
        build-be build-ui clean kill db-up db-down db-clean

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
	( cargo run --bin rubix-server $(CARGO_FEATURES) $(SEED_FLAG) ) & \
	( cd $(UI_DIR) && pnpm dev ) & \
	wait

dev-be:
	cargo run --bin rubix-server $(CARGO_FEATURES) $(SEED_FLAG)

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
