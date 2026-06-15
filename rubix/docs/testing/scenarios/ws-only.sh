#!/usr/bin/env bash
# Focused live-query (WebSocket) check against an ALREADY-seeded store.
#
# Usage:
#   DATA=/tmp/rubix-e2e docs/testing/scenarios/ws-only.sh
#
# Boots the server WITHOUT --seed-dev against an existing data dir (fast — no
# re-seed), opens /ws/records with auth headers, inserts a record, and asserts a
# live event arrives. Run e2e.sh first (it leaves the seeded store under DATA).
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUBIX_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$RUBIX_DIR"

PORT="${PORT:-8095}"
B="http://127.0.0.1:$PORT"
DATA="${DATA:-/tmp/rubix-e2e}"
LOG="${LOG:-/tmp/rubix-ws-only.log}"

if [ ! -d "$DATA" ]; then echo "no seeded data dir at $DATA — run e2e.sh first (or set DATA)"; exit 1; fi

pkill -9 -f 'rubix-server' 2>/dev/null; sleep 1
RUBIX_DATA_DIR="$DATA" RUBIX_BIND="127.0.0.1:$PORT" \
  cargo run -q -p rubix-server --bin rubix-server > "$LOG" 2>&1 &
SRV=$!
for i in $(seq 1 60); do
  curl -s "$B/health" >/dev/null 2>&1 && break
  kill -0 "$SRV" 2>/dev/null || { echo "server died"; cat "$LOG"; exit 1; }
  sleep 1
done
echo "### server up; acme record count:"
curl -s "$B/records" -H 'x-rubix-subject: acme_viewer' -H 'x-rubix-secret: viewer-demo' | jq 'length'
echo "### ws test (with auth headers):"
node "$SCRIPT_DIR/ws-test.js" "$PORT"; WS=$?
echo "ws-exit=$WS"
kill -9 "$SRV" 2>/dev/null
[ "$WS" -eq 0 ]
