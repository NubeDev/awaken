#!/usr/bin/env bash
# One wake of the rubix fleet/dashboard build loop. Fired by cron every 5 minutes (see install-cron.sh).
# Acquires an exclusive lock so overlapping firings can't double-spawn a workstream, honors the
# STOP sentinel kill switch, then runs ONE pass of the LOOP ALGORITHM in _ORCHESTRATION.md headless.
set -euo pipefail

# Cron runs with a stripped PATH that lacks claude/cargo/git — without this the tick dies with
# "claude: command not found" and the whole loop silently no-ops. Pin the real tool dirs so the
# tick and every subagent (which shell out to cargo/git/pnpm) find their binaries.
export PATH="/home/user/snap/code/226/.local/share/pnpm:/home/user/.cargo/bin:/home/user/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
CLAUDE_BIN="/home/user/snap/code/226/.local/share/pnpm/claude"

REPO="/home/user/code/rust/awaken"
SESS="$REPO/rubix/docs/sessions/june-13th"
LOG="$SESS/cron.log"
LOCK="$SESS/.loop.lock"
STOP="$SESS/.loop.STOP"

ts() { date -u +%FT%TZ; }

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
# is confined to this repo on branch rubix-gaps. --model pins Opus 4.8 for the tick AND every
# subagent (subagents inherit the parent model). Effort level comes from ~/.claude/settings.json.
"$CLAUDE_BIN" -p "Read rubix/docs/sessions/june-13th/_ORCHESTRATION.md (the LOOP ALGORITHM + AGENT CHARTER) and rubix/docs/sessions/june-13th/STATUS.md (the queue). Execute exactly ONE wake of the LOOP ALGORITHM (headless cron mode), then exit. You are on branch rubix-gaps. Do not ask questions; a blocked workstream logs to rubix/docs/sessions/june-13th/TODOs.md and the next pending one is chosen. Step 2 guard: if the first non-pending row is in-progress but git status shows NO uncommitted changes to its owned files and no subagent is running, treat it as returned and run the DONE GATE on it; only skip if work is genuinely in-flight. Spawn the workstream session as a subagent using the AGENT CHARTER verbatim, substituting the WS-xx number and pointing it at rubix/docs/sessions/june-13th/WS-xx.md as its spec and the design doc that WS cites. Use the DONE GATE matching the WS type (cargo for backend, pnpm -C rubix/ui build + test:unit for frontend, BOTH for a WS that spans both). Commit only files the workstream owns. When you have spawned or gated exactly one workstream, append a one-line entry to rubix/docs/sessions/june-13th/STATUS.md's loop log and stop." \
  --model claude-opus-4-8 \
  --dangerously-skip-permissions \
  >>"$LOG" 2>&1

echo "$(ts) wake-complete pid=$$" >"$HEARTBEAT"
echo "$(ts) wake complete." >>"$LOG"
