.PHONY: dev dev-ui dev-agent

# Required: OPENAI_API_KEY, AWAKEN_ADMIN_API_BEARER_TOKEN
#   (admin/config/trace/eval APIs require the bearer token whenever exposed)
# Optional overrides:
#   OPENAI_BASE_URL   — custom provider base URL
#   AGENT_MODEL       — model name (default: deepseek-chat)
#   AWAKEN_HTTP_ADDR  — backend bind address (default: 127.0.0.1:38080)
#   VITE_BACKEND_URL  — frontend backend URL (default: http://localhost:38080)
#   VITE_AGENT_ID     — agent ID (default: openui-ui)
#
# Copy .env.example to .env and fill in values; the targets below load it.

# Load .env into the environment for every target if it exists.
ifneq (,$(wildcard .env))
include .env
export
endif

# Runs both frontend (vite :3002) and backend agent concurrently.
dev:
	npm run dev

dev-ui:
	npm run dev:ui

dev-agent:
	cargo run -p ai-sdk-starter-agent
