#!/usr/bin/env bash
# One wake of the NHP POC build loop. Fired by cron every 5 minutes (see install-cron.sh).
# Acquires an exclusive lock so overlapping firings can't double-spawn a workstream, honors the
# STOP sentinel kill switch, then runs ONE pass of the LOOP ALGORITHM in setup/_ORCHESTRATION.md
# headless.
set -euo pipefail

# Cron runs with a stripped PATH that lacks claude/node/pnpm/cargo/git — without this the tick dies
# with "command not found" and the whole loop silently no-ops. Pin the real tool dirs so the tick
# and every subagent (which shell out to pnpm/cargo/git) find their binaries. The node dir is the
# nvm install on this box; cargo is ~/.cargo/bin. (See nhp/docs/sessions setup notes.)
export PATH="/var/config/nvm/versions/node/v22.22.3/bin:/home/user/.cargo/bin:/home/user/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

REPO="/home/user/code/rust/awaken"
SESS="$REPO/nhp/docs/sessions"
LOG="$SESS/cron.log"
LOCK="$SESS/.loop.lock"
STOP="$SESS/.loop.STOP"

ts() { date -u +%FT%TZ; }

# Resolve the claude CLI: prefer $CLAUDE_BIN, else PATH, else a couple of common spots. Fail loudly
# (into the log) rather than silently no-op if it can't be found.
CLAUDE_BIN="${CLAUDE_BIN:-$(command -v claude || true)}"
if [[ -z "$CLAUDE_BIN" ]]; then
  for c in /var/config/nvm/versions/node/v22.22.3/bin/claude "$HOME/.local/bin/claude" /usr/local/bin/claude; do
    [[ -x "$c" ]] && { CLAUDE_BIN="$c"; break; }
  done
fi
if [[ -z "$CLAUDE_BIN" ]]; then
  echo "$(ts) ERROR: claude CLI not found on PATH — set CLAUDE_BIN in this script or install claude." >>"$LOG"
  exit 1
fi

# Kill switch: `touch .loop.STOP` to halt the run without editing crontab.
if [[ -f "$STOP" ]]; then
  echo "$(ts) STOP sentinel present — exiting without spawning." >>"$LOG"
  exit 0
fi

# Single-firing lock. -n = fail immediately if another firing holds it. The kernel releases this on
# process death (even SIGKILL), so a held lock ALWAYS means a live holder — never rm the lock file.
exec 9>"$LOCK"
if ! flock -n 9; then
  echo "$(ts) another firing holds the lock — skip." >>"$LOG"
  exit 0
fi

cd "$REPO"
echo "$(ts) firing one wake." >>"$LOG"

# Heartbeat: records THIS firing's PID so a watcher can tell alive-vs-dead with `kill -0 <pid>`.
HEARTBEAT="$SESS/.loop.heartbeat"
echo "$(ts) wake-start pid=$$" >"$HEARTBEAT"

# One headless wake. Claude reads the driver doc, runs the LOOP ALGORITHM once, spawns/gates one WS,
# updates STATUS, and exits. --dangerously-skip-permissions because cron is non-interactive; the work
# is confined to this repo on branch nhp-poc. --model pins Opus 4.8 for the tick AND every subagent
# (subagents inherit the parent model). Effort level comes from ~/.claude/settings.json.
"$CLAUDE_BIN" -p "Read nhp/docs/sessions/setup/_ORCHESTRATION.md (the LOOP ALGORITHM + AGENT CHARTER) and nhp/docs/sessions/STATUS.md (the workstream queue). Execute exactly ONE wake of the LOOP ALGORITHM (headless cron mode), then exit. You are on branch nhp-poc — if it does not exist yet, create it from the current HEAD before spawning the first workstream, then stay on it. Do not ask questions; a blocked workstream logs to nhp/docs/sessions/TODOs.md and the next pending one is chosen. Step 2 guard: if the first non-pending row is in-progress but git status shows NO uncommitted changes to its owned files and no subagent is running, treat it as returned and run the DONE GATE on it; only skip if work is genuinely in-flight. Spawn the workstream session as a subagent using the AGENT CHARTER verbatim, substituting the WS-xx number and pointing it at nhp/docs/sessions/WS-xx.md as its spec. Use the DONE GATE from _ORCHESTRATION step 4 (UI: pnpm -C nhp/ui build + test; backend-touching: cargo gate; seed: the make/seed command). Commit only files the workstream owns. When you have spawned or gated exactly one workstream, append a one-line entry to nhp/docs/sessions/STATUS.md's loop log and stop." \
  --model claude-opus-4-8 \
  --dangerously-skip-permissions \
  >>"$LOG" 2>&1

echo "$(ts) wake-complete pid=$$" >"$HEARTBEAT"
echo "$(ts) wake complete." >>"$LOG"
