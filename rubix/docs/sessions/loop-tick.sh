#!/usr/bin/env bash
# One wake of the rubix backend build loop. Fired by cron every 5 minutes (see install-cron.sh).
# Acquires an exclusive lock so overlapping firings can't double-spawn a workstream, honors the
# STOP sentinel kill switch, then runs ONE pass of the LOOP ALGORITHM in _ORCHESTRATION.md headless.
set -euo pipefail

# Cron runs with a stripped PATH that lacks claude/cargo/git — without this the tick dies with
# "claude: command not found" and the whole loop silently no-ops. Pin the real tool dirs so the
# tick and every subagent (which shell out to cargo/git) find their binaries.
export PATH="/home/user/snap/code/226/.local/share/pnpm:/home/user/.cargo/bin:/home/user/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
CLAUDE_BIN="/home/user/snap/code/226/.local/share/pnpm/claude"

REPO="/home/user/code/rust/awaken"
SESS="$REPO/rubix/docs/sessions"
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
# is confined to this repo on branch new-rubix. --model pins Opus 4.8 for the tick AND every subagent
# (subagents inherit the parent model). Effort level comes from ~/.claude/settings.json.
"$CLAUDE_BIN" -p "Read rubix/docs/sessions/_ORCHESTRATION.md (the LOOP ALGORITHM + AGENT CHARTER) and rubix/docs/sessions/STATUS.md (the workstream queue). Execute exactly ONE wake of the LOOP ALGORITHM (headless cron mode), then exit. You are on branch new-rubix — do NOT switch or create branches. Do not ask questions; a blocked workstream logs to rubix/docs/sessions/TODOs.md, sets its row to blocked, and the next pending one is chosen. Step 2 guard: if the first non-pending row is in-progress but git log shows no new commits across recent firings AND its WS-xx.md Status is not Done/Blocked, treat the subagent as dead and re-spawn the SAME WS fresh (work is idempotent); if WS-xx.md says Done/Blocked, run the DONE GATE / honor the block; only skip if work is genuinely in-flight (commits advancing). When you spawn the next pending WS, set its row to in-progress with a real 'date -u' Started timestamp, append a loop-log line, then spawn the workstream as a subagent using the AGENT CHARTER from _ORCHESTRATION.md VERBATIM, substituting the WS number and pointing the subagent at rubix/docs/sessions/WS-xx.md as its spec. Use the cargo DONE GATE (cd rubix && cargo test --workspace green + cargo clippy --workspace --all-targets clean + the WS-xx.md Done line + committed on new-rubix with a WS-xx: prefixed message). Commit only files the workstream owns (git add -p), never blind git add -A. When you have spawned OR gated exactly one workstream, append a one-line entry to the rubix/docs/sessions/STATUS.md loop log and stop." \
  --model claude-opus-4-8 \
  --dangerously-skip-permissions \
  >>"$LOG" 2>&1

echo "$(ts) wake-complete pid=$$" >"$HEARTBEAT"
echo "$(ts) wake complete." >>"$LOG"
