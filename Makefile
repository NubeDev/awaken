# Awaken dev stack Makefile
#
#   make install   - install BE (cargo build) + FE (pnpm) deps
#   make dev       - install (if needed) and run backend + admin console
#   make kill      - stop everything started by `make dev`
#   make logs      - tail backend + UI logs
#   make status    - show what's running
#
# Backend:  http://127.0.0.1:38080   (admin bearer token: dev-token)
# UI:       http://127.0.0.1:3002

SHELL := /bin/bash

# --- config -----------------------------------------------------------------
HTTP_ADDR        ?= 127.0.0.1:38080
ADMIN_TOKEN      ?= dev-token
STORAGE_DIR      ?= ./target/awaken-dev
BE_PACKAGE       ?= ai-sdk-starter-agent
UI_FILTER        ?= awaken-admin-console

RUN_DIR := .dev
BE_PID  := $(RUN_DIR)/backend.pid
UI_PID  := $(RUN_DIR)/ui.pid
BE_LOG  := $(RUN_DIR)/backend.log
UI_LOG  := $(RUN_DIR)/ui.log

# Optional: pass an LLM key through, e.g. `make dev OPENAI_API_KEY=sk-...`
BE_ENV := AWAKEN_HTTP_ADDR=$(HTTP_ADDR) \
          AWAKEN_ADMIN_API_BEARER_TOKEN=$(ADMIN_TOKEN) \
          AWAKEN_STORAGE_DIR=$(STORAGE_DIR)

.PHONY: help install install-be install-fe build dev backend ui kill logs status clean

help:
	@echo "Awaken dev stack:"
	@echo "  make install   install backend + frontend deps"
	@echo "  make dev       run backend + admin console (installs first)"
	@echo "  make kill      stop backend + admin console"
	@echo "  make logs      tail logs"
	@echo "  make status    show running services"
	@echo ""
	@echo "  Backend: http://$(HTTP_ADDR)  (token: $(ADMIN_TOKEN))"
	@echo "  UI:      http://127.0.0.1:3002"

# --- install ----------------------------------------------------------------
install: install-be install-fe

install-be:
	@echo ">> Building backend ($(BE_PACKAGE))..."
	cargo build -p $(BE_PACKAGE)

install-fe:
	@echo ">> Installing frontend deps..."
	pnpm install

build: install-be
	@echo ">> Building frontend..."
	pnpm --filter $(UI_FILTER) build

# --- run --------------------------------------------------------------------
dev: install backend ui
	@echo ""
	@echo ">> Stack is up."
	@echo "   Backend: http://$(HTTP_ADDR)  (token: $(ADMIN_TOKEN))"
	@echo "   UI:      http://127.0.0.1:3002"
	@echo "   Logs:    make logs   |   Stop: make kill"

$(RUN_DIR):
	@mkdir -p $(RUN_DIR)

# Each service is launched with `setsid` so it becomes a process-group leader.
# We record the PGID (== leader pid) and on `kill` signal the whole group with
# `kill -- -PGID`, which reaps cargo/pnpm and every child they spawn. No
# `pkill -f` pattern matching (which would also catch the make process itself).
backend: $(RUN_DIR)
	@if [ -f $(BE_PID) ] && kill -0 $$(cat $(BE_PID)) 2>/dev/null; then \
		echo ">> Backend already running (pid $$(cat $(BE_PID)))"; \
	else \
		echo ">> Starting backend on $(HTTP_ADDR)..."; \
		$(BE_ENV) setsid cargo run -p $(BE_PACKAGE) > $(BE_LOG) 2>&1 < /dev/null & echo $$! > $(BE_PID); \
		echo "   pid $$(cat $(BE_PID)), log $(BE_LOG)"; \
	fi

ui: $(RUN_DIR)
	@if [ -f $(UI_PID) ] && kill -0 $$(cat $(UI_PID)) 2>/dev/null; then \
		echo ">> UI already running (pid $$(cat $(UI_PID)))"; \
	else \
		echo ">> Starting admin console on http://127.0.0.1:3002..."; \
		setsid pnpm --filter $(UI_FILTER) dev > $(UI_LOG) 2>&1 < /dev/null & echo $$! > $(UI_PID); \
		echo "   pid $$(cat $(UI_PID)), log $(UI_LOG)"; \
	fi

# --- stop -------------------------------------------------------------------
kill:
	@echo ">> Stopping stack..."
	@for f in $(BE_PID) $(UI_PID); do \
		if [ -f $$f ]; then \
			pid=$$(cat $$f); \
			if kill -0 $$pid 2>/dev/null; then \
				kill -- -$$pid 2>/dev/null || kill $$pid 2>/dev/null || true; \
				echo "   killed process group $$pid ($$f)"; \
			fi; \
			rm -f $$f; \
		fi; \
	done
	@echo ">> Stopped."

# --- utils ------------------------------------------------------------------
logs:
	@tail -n 40 -F $(BE_LOG) $(UI_LOG)

status:
	@for name in backend ui; do \
		pidf=$(RUN_DIR)/$$name.pid; \
		if [ -f $$pidf ] && kill -0 $$(cat $$pidf) 2>/dev/null; then \
			echo "$$name: running (pid $$(cat $$pidf))"; \
		else \
			echo "$$name: stopped"; \
		fi; \
	done

clean: kill
	rm -rf $(RUN_DIR)
