# NHP POC — Orchestration Loop (the driver)

> This is the script the **loop** follows on every wake. It is NOT a workstream.
> The loop is the parent session; each workstream runs as a fresh **subagent** spawned by the loop.
> Everything lands on branch **`nhp-poc`**, sequentially. No worktrees, no parallel writers.
>
> **POC speed mode.** This is a proof-of-concept — the goal is to *smash it out* and get a working
> end-to-end NHP (collections + seed + admin + wizards + dashboards) on top of the already-built
> rubix backend. Bias to shipping over polish: prefer the simplest thing that works, reuse rubix +
> rubix-old/ui code aggressively, and don't gold-plate. Production hardening is explicitly out of
> scope for the POC unless a WS spec says otherwise.
>
> **NOTE — another AI session may be running concurrently.** Don't stress about diffs you didn't
> write. If a file you need to commit also has someone else's unrelated changes, commit only the
> hunks your WS touched (`git add -p`), and never revert/clobber changes you didn't make.

## Scope

NHP — a power-metering management platform built as a thin domain + UI layer on the **existing**
rubix backend. The product scope is [nhp/docs/OVERVIEW.md](../../OVERVIEW.md) and the four feature
docs beside it ([DOMAIN-MODEL.md](../../DOMAIN-MODEL.md), [ADMIN.md](../../ADMIN.md),
[WIZARDS.md](../../WIZARDS.md), [DASHBOARDS.md](../../DASHBOARDS.md), [SEED.md](../../SEED.md)). The
file-layout standard every session obeys is
[rubix/docs/FILE-LAYOUT.md](../../../../rubix/docs/FILE-LAYOUT.md). The queue in
[STATUS.md](../STATUS.md) decomposes the product into workstreams.

NHP does **not** rebuild the rubix backend — it is already done (verified in
[OVERVIEW.md](../../OVERVIEW.md) build-status). NHP *consumes* it: collection definitions, a seed,
and a UI ported from `rubix-old/ui`. The only backend touches are the three gaps OVERVIEW lists
(Select field type, prefs endpoint, file blobs) and only if a WS calls for them.

## Why sequential on one branch

Parallel agents on one branch overwrite each other. Sequential on one branch means each session
**commits before the next starts**, so a later session finds its dependencies (e.g. the collection
definitions, or the seeded data) already sitting in the working tree — dependencies resolve for
free, no merging. The cost is wall-clock; the win is reliability and zero collisions. This is the
explicit user choice.

---

## LOOP ALGORITHM (run this every wake)

1. **Read [STATUS.md](../STATUS.md).** Identify the queue and each WS's status.
2. **Is a WS currently 🔵 in-progress?**
   - If a subagent is still running for it → do nothing, reschedule, exit. (Don't double-spawn.)
   - If marked 🔵 but no subagent is running (it returned) → run the **DONE GATE** on it (step 4).
3. **No WS in progress?** Pick the **first** WS in queue order whose status is ⬜ pending.
   - If none pending → check for ⛔ blocked rows whose blocker the human has since resolved
     (TODOs.md entry struck through / dated Resolution): reset those to ⬜ and pick the first.
   - If everything is ✅ or ⛔ and nothing is unblockable → **the run is complete.** Write a final
     loop-log line, summarize, and STOP the loop (do not reschedule).
4. **DONE GATE** (before marking any WS ✅ — this is how we trust a session finished):
   - The WS's **own gate commands pass** (each WS doc states them; default below).
   - **UI workstreams:** `pnpm -C nhp/ui build` succeeds and `pnpm -C nhp/ui test` (unit) is green
     if the WS added tests. Typecheck (`tsc`/`vite build`) must pass — no `// @ts-ignore` to dodge.
   - **No rubix diff:** `git status` must show ZERO changes under `rubix/` (NHP is frozen against
     rubix). The only exception is a brand-new `rubix-ext` extension a WS spec explicitly authorised
     — if present, `cd rubix && cargo test --workspace` green + `cargo clippy --workspace
     --all-targets` clean. Any other diff under `rubix/` fails the gate.
   - **Data/seed workstreams:** the documented `make`/seed command runs clean and produces the
     expected records (the WS doc says how to verify).
   - The session wrote a **`Done`** status line in its own `sessions/WS-xx.md` with a finish timestamp.
   - Working tree changes are **committed** on `nhp-poc` with a `WS-xx:` prefixed message.
   - If all pass → mark the row ✅, fill Finished + Commit columns, append a loop-log line.
   - If the build/tests are **red** and the session didn't flag a blocker → the session is NOT done.
     Spawn a fresh subagent to *fix that WS only* (same charter). Do not advance.
5. **Spawn the next session** (step 3's pick): set its row to 🔵, fill Started, append a loop-log
   line, then launch the subagent with the **AGENT CHARTER** below (substituting the WS number).
6. **Reschedule** the next wake (~5 min) and exit. The loop re-enters at step 1.

> The loop itself never writes feature code. It only: reads STATUS, runs the gate, spawns one
> subagent, updates STATUS, reschedules. All feature work happens inside subagents.

---

## AGENT CHARTER (paste into every spawned subagent, substitute <WS-xx>)

```
You are implementing <WS-xx> for the NHP power-metering POC, as one autonomous session in an
unattended build. You run to completion and return — you cannot ask the human anything. This is a
POC: smash it out, reuse aggressively, prefer the simplest thing that works. Do not gold-plate.

READ FIRST, IN ORDER:
1. nhp/docs/OVERVIEW.md                             (the product + what rubix already gives you)
2. rubix/docs/FILE-LAYOUT.md                        (the file-layout standard — governs every file)
3. nhp/docs/{DOMAIN-MODEL,ADMIN,WIZARDS,DASHBOARDS,SEED}.md  (whichever your WS touches)
4. nhp/docs/sessions/WS-xx.md                       (your spec — source of truth for scope)
5. nhp/docs/sessions/STATUS.md                      (what's already done — your deps are committed)

WHAT NHP IS (don't re-derive): a thin domain + UI layer on the ALREADY-BUILT rubix backend. You
define collections (records, not tables), seed mock data, and build UI ported from rubix-old/ui.
You do NOT rebuild rubix. You do NOT talk to hardware (no Modbus, no polling) — that's a separate
service that consumes NHP's stored register metadata. Status/last_seen/live values are written by
that poller; in the POC the SEED workstream fakes them.

STANDARD (FILE-LAYOUT.md is load-bearing — the rules that bite hardest):
- ONE RESPONSIBILITY PER FILE, ≤400 lines (hard ceiling), ~100 typical. Verb-per-file folders
  (create.ts/list.ts/update.ts), not one noun-file-does-everything. No utils/helpers/common/misc —
  name the concept. mod.rs / index.ts are barrels only (re-exports, no logic).
- Search the repo FIRST for related/similar code; reuse or refactor to dedupe before adding new
  code. rubix-old/ui and rubix/ui are your parts bin — copy and adapt, don't reinvent.
- No placeholder impls that pretend to work, no fallbacks that hide failures. Blocked? Log a TODO.
- Comments explain WHY not WHAT. No progress markers (// STAGE-1, // FIXED:), no emoji in code.
  Bare TODOs forbidden — use `// TODO(loop):`. Code comments reference nhp/docs or rubix/docs only,
  never these session docs.

HARD RULES (this is an unattended run — violating these poisons every later session):
- BRANCH: work on `nhp-poc`. Do NOT create branches or worktrees. Do NOT switch branches.
  Another AI session may be editing the same branch — commit only YOUR hunks (`git add -p` the
  files your WS owns), never blind `git add -A`, never revert changes you didn't make.
- NO QUESTIONS: you cannot prompt the human. If you hit a genuine ambiguity or need work a
  not-yet-run session owns, you DO NOT guess and DO NOT hack/stub. Instead:
    (a) append a dated entry to nhp/docs/sessions/TODOs.md in the documented format,
    (b) set your row in STATUS.md to ⛔ blocked with a one-line reason,
    (c) commit whatever works so far, then STOP and return a summary.
- NO HACKS to "make it pass": no dodging typecheck with @ts-ignore, no #[ignore]/skipped tests, no
  commented-out tests, no narrowing a test to pass. Can't do it properly → TODO entry, not a fake.
- STAY IN YOUR LANE: edit the files your WS owns. Touch a shared file (a router barrel, the app
  shell, a shared types file, the Makefile) only as a minimal additive change. If a contract you
  depend on is missing, that's a TODOs.md blocker — do not redefine it.
- INCREMENTAL: implement one logical section, verify it, commit, repeat. Don't dump one giant commit.
- CLEAN UP TEST ARTIFACTS: if you boot a throwaway rubix backend to smoke-test, its SurrealDB writes
  to a `rubix-data/` dir. Point it inside `rubix/` (already gitignored as `/rubix-data`) via the data
  path env/flag, OR `rm -rf` the stray `rubix-data/` before you finish. Never leave an untracked
  `rubix-data/` at the worktree root, and never commit one.
- Keep your WS's DONE GATE green before you call yourself done (see the gate in your WS doc /
  _ORCHESTRATION step 4). A red build/typecheck/test means you are NOT done.
- RUBIX IS FROZEN — NHP NEVER EDITS RUBIX SOURCE. NHP is UI + data on the already-built rubix
  binary, unchanged. If a WS finds it needs Rust:
    * Generic & reusable by others (e.g. a Select/enum field type, a new core capability) → do NOT
      implement it. File a **task for the rubix team** in nhp/docs/sessions/TODOs.md (it needs their
      approval) and take the NHP-only workaround (data + writeRule + UI) for the POC. This is a
      TODOs.md blocker entry titled `RUBIX-TEAM:`, not a code change.
    * NHP-specific Rust behaviour → it belongs in a **rubix extension** (rubix-ext: a scoped
      principal, control via JSON-RPC, data via Zenoh — see rubix/crates/rubix-ext/README.md), NOT
      in rubix core. For the POC, only build an extension if a WS spec explicitly calls for one;
      otherwise log it.
  A DONE GATE FAILS if `git status` shows ANY diff under `rubix/` that isn't a brand-new extension a
  WS spec authorised. Touching rubix core/crates from an NHP workstream is forbidden.

SESSION LOG (mandatory): create/maintain nhp/docs/sessions/WS-xx.md with:
  - a `Status:` line (In-progress / Blocked / Done) and a `Started:` + `Finished:` UTC timestamp,
  - the task breakdown you executed and what each commit did,
  - any assumptions, any deviations, any follow-ups.

FIRST ACTION (mandatory): re-check any file:line your WS's "Current state" section cites against the
real tree (rubix and rubix-old/ui drift); if a claim is wrong, fix the WS doc + bump its `Verified:`
line BEFORE coding. Then confirm your dependency WSs are committed. Then implement incrementally,
commit (messages prefixed `<WS-xx>:`), and update STATUS.md + your session doc. When done, ensure
your gate is green and return a concise summary of what landed and what (if anything) you logged to
TODOs.md.
```

---

## HEADLESS CRON MODE (the 100%-unattended path)

The loop survives a closed editor / sleeping session only when fired by the OS, not from a chat
window. The cron job runs **one wake per firing** with `claude -p` and exits — it is NOT the
in-session `/loop`. Each firing executes the LOOP ALGORITHM above exactly once.

**Concurrency lock (MANDATORY — prevents two firings double-spawning a WS):**
Before doing anything, the firing acquires an exclusive lock and skips if it can't:
```
exec 9>nhp/docs/sessions/.loop.lock
flock -n 9 || { echo "$(date -u +%FT%TZ) another firing holds the lock — skip"; exit 0; }
```
A firing that holds the lock runs ONE wake (gate the returned WS, or spawn the next pending WS) and
exits, releasing the lock. A WS subagent can run longer than 5 min; that's fine — subsequent firings
see the row is 🔵 with work still committing and either (a) the subagent already returned → run the
gate, or (b) detect no new commits + no completion in WS-xx.md for a while → treat as still-running
and skip. **Never spawn a second WS while one is 🔵 and its WS-xx.md has no Blocked/Done line.**

**Determining "subagent still running" without live process state:** headless firings can't see a
previous firing's subagent. Use durable signals only: the WS-xx.md `Status:` line and `git log`.
- Row 🔵 + WS-xx.md Status `In-progress` + commits advancing across firings → still working, skip.
- Row 🔵 + WS-xx.md Status `Done`/`Blocked` → run the gate / honor the block, then advance.
- Row 🔵 + WS-xx.md Status `In-progress` + NO new commits for ≥3 firings (~15 min) → assume the
  subagent died; re-spawn the SAME WS fresh (it resumes from committed state — work is idempotent
  because each WS reads STATUS + git to see what's already landed).

**The kernel-backed lock is the real mutex.** `loop-tick.sh` writes `.loop.heartbeat`
(`<utc> wake-start pid=<pid>`) before the long claude call and `wake-complete` after. flock on fd 9
is released by the kernel when the holder dies (even SIGKILL), so a held lock ALWAYS means a live
holder — never `rm` the lock to "recover". The heartbeat only lets a watcher identify the owning PID.

**The installer:** `setup/install-cron.sh` writes the crontab line. To stop the run, the human runs
`./install-cron.sh remove` (or `crontab -r`). Kill switch without a crontab edit: a file
`sessions/.loop.STOP` makes every firing exit immediately without spawning.

## Notes for the loop driver
- **One subagent at a time.** Never spawn a second WS while one is 🔵 with a live subagent.
- **Timestamps:** the runtime has no clock inside scripts; when you (the loop) write timestamps,
  use `date -u` via Bash to get the real UTC time.
- **Crash recovery:** if the loop is restarted, step 1 reconstructs all state from STATUS.md +
  the per-session docs + `git log` — there is no hidden state. Safe to resume any time.
- **Definition of "all done":** every queue row is ✅, OR the remaining rows are ⛔ blocked and
  their TODOs are unresolved. Then STOP and report.
