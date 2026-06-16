#!/usr/bin/env bash
# End-to-end smoke for the NHP POC — proves it runs front to back from a clean
# checkout, on the UNMODIFIED rubix binary (rubix is frozen).
#
# It wires up the existing checks into ONE entrypoint rather than inventing new
# ones (WS-08): boot rubix --seed-dev → register collections → seed the portfolio
# → seed-check → records-check (the dashboard data pipeline) → pnpm build →
# pnpm test:unit. Each step gates the next; any failure aborts non-zero.
#
# Run from the nhp/ dir (or via `make smoke`, which sets RUBIX_BASE/ports):
#   bash scripts/smoke.sh
#
# The throwaway backend writes its data dir INSIDE rubix/ (gitignored
# /rubix-data) and is killed + removed on exit, so the worktree root never grows a
# stray rubix-data/ and rubix's tree stays clean.
set -euo pipefail

# Resolve paths relative to this script so it runs from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NHP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUBIX_DIR="$(cd "$NHP_DIR/../rubix" && pwd)"
UI_DIR="$NHP_DIR/ui"

# Ports/creds mirror nhp/Makefile + .env.example. A throwaway data dir inside
# rubix/ (gitignored) keeps the worktree root clean.
BE_PORT="${BE_PORT:-8094}"
BASE="http://127.0.0.1:${BE_PORT}"
DATA_DIR="$RUBIX_DIR/rubix-data"
export RUBIX_BASE="$BASE"
export RUBIX_BIND="127.0.0.1:${BE_PORT}"
export RUBIX_SUBJECT="${RUBIX_SUBJECT:-acme_operator}"
export RUBIX_SECRET="${RUBIX_SECRET:-operator-demo}"

# node/pnpm are under nvm, not on PATH (see project memory); cargo under ~/.cargo.
export PATH="/var/config/nvm/versions/node/v22.22.3/bin:$HOME/.cargo/bin:$PATH"

BE_PID=""
cleanup() {
  [ -n "$BE_PID" ] && kill "$BE_PID" 2>/dev/null || true
  wait "$BE_PID" 2>/dev/null || true
  rm -rf "$DATA_DIR"
}
trap cleanup EXIT INT TERM

step() { echo; echo "── $* ──"; }

step "1/7  boot rubix --seed-dev (throwaway data dir in rubix/, removed on exit)"
rm -rf "$DATA_DIR"
( cd "$RUBIX_DIR" && RUBIX_DATA_DIR="$DATA_DIR" cargo run --quiet --bin rubix-server -- --seed-dev ) &
BE_PID=$!

# Wait for the records API to answer (the seeded namespace + principals are ready
# once /records responds to the operator credential). 90s budget for a cold boot
# + the --seed-dev portfolio write before the HTTP listener accepts.
echo "   waiting for $BASE/records …"
for i in $(seq 1 90); do
  if curl -sf -o /dev/null \
      -H "x-rubix-subject: $RUBIX_SUBJECT" -H "x-rubix-secret: $RUBIX_SECRET" \
      "$BASE/records?kind=tenant"; then
    echo "   backend up after ${i}s"
    break
  fi
  if ! kill -0 "$BE_PID" 2>/dev/null; then
    echo "   backend exited during boot" >&2; exit 1
  fi
  sleep 1
done

step "2/7  register the 7 NHP collection definitions"
node "$NHP_DIR/collections/register-collections.mjs"

step "3/7  seed the mock portfolio + faked poller data"
node "$NHP_DIR/seed/seed.mjs"

step "4/7  seed-check (portfolio counts present)"
node "$NHP_DIR/seed/check.mjs"

step "5/7  records-check (dashboard data pipeline returns rows)"
node "$NHP_DIR/seed/records-check.mjs"

step "6/7  build the UI (pnpm install + tsc + vite build)"
( cd "$UI_DIR" && pnpm install --frozen-lockfile && pnpm build )

step "7/7  unit tests (pnpm test:unit)"
( cd "$UI_DIR" && pnpm test:unit )

echo
echo "SMOKE PASSED — boot → collections → seed → seed-check → records-check → build → unit tests"
