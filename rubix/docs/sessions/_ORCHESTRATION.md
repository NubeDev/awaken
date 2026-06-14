# Rubix Backend — Orchestration Loop (the driver)

> This is the script the **loop** follows on every wake. It is NOT a workstream.
> The loop is the parent session; each workstream runs as a fresh **subagent** spawned by the loop.
> Everything lands on branch **`new-rubix`**, sequentially. No worktrees, no parallel writers.
>
> **NOTE — another AI session may be running concurrently.** Don't stress about diffs you didn't
> write. If a file you need to commit also has someone else's unrelated changes, commit only the
> hunks your WS touched (`git add -p`), and never revert/clobber changes you didn't make.

## Scope

The rubix edge-to-cloud data platform backend (`rubix/`, a standalone Cargo workspace, not part of
the awaken workspace). The build target is [STACK-DEISGN.md](../../STACK-DEISGN.md) and the scope
authority is [SCOPE.md](../SCOPE.md); the gap inventory is [STATUS.md](../../STATUS.md) ("Not started
/ remaining"). The queue in [STATUS.md](./STATUS.md) (this directory) decomposes those gaps into
workstreams. The build is greenfield — WS-01 stands up the Cargo workspace; every later WS finds its
dependencies already committed in the tree.

## Why sequential on one branch

Parallel agents on one branch overwrite each other. Sequential on one branch means each session
**commits before the next starts**, so a later session finds its dependencies (e.g. WS-01's store)
already sitting in the working tree — dependencies resolve for free, no merging. The cost is
wall-clock; the win is reliability and zero collisions. This is the explicit user choice.

---

## LOOP ALGORITHM (run this every wake)

1. **Read [STATUS.md](./STATUS.md).** Identify the queue and each WS's status.
2. **Is a WS currently 🔵 in-progress?**
   - If a subagent is still running for it → do nothing, reschedule, exit. (Don't double-spawn.)
   - If marked 🔵 but no subagent is running (it returned) → run the **DONE GATE** on it (step 4).
3. **No WS in progress?** Pick the **first** WS in queue order whose status is ⬜ pending.
   - If none pending → check for ⛔ blocked rows whose blocker the human has since resolved
     (TODOs.md entry struck through / removed): reset those to ⬜ and pick the first.
   - If everything is ✅ or ⛔ and nothing is unblockable → **the run is complete.** Write a final
     loop-log line, summarize, and STOP the loop (do not reschedule).
4. **DONE GATE** (before marking any WS ✅ — this is how we trust a session finished):
   - `cd rubix && cargo test --workspace` is **green**.
   - `cd rubix && cargo clippy --workspace --all-targets` is **clean** (no warnings).
   - If DTOs/handlers changed, the OpenAPI surface still builds (`/api-docs/openapi.json` route
     compiles; utoipa derives are in sync). (Only applies once WS-16 transport exists.)
   - The session wrote a **`Done`** status line in its own `sessions/WS-xx.md` with a finish timestamp.
   - Working tree changes are **committed** on `new-rubix` with a `WS-xx:` prefixed message
     following the commit convention (`<emoji> <type>(<scope>): <subject>`, see CLAUDE.md).
   - If all pass → mark the row ✅, fill Finished + Commit columns, append a loop-log line.
   - If the build/tests are **red** and the session didn't flag a blocker → the session is NOT done.
     Spawn a fresh subagent to *fix the build for that WS only* (same charter). Do not advance.
5. **Spawn the next session** (step 3's pick): set its row to 🔵, fill Started, append a loop-log
   line, then launch the subagent with the **AGENT CHARTER** below (substituting the WS number).
6. **Reschedule** the next wake (~5 min) and exit. The loop re-enters at step 1.

> The loop itself never writes feature code. It only: reads STATUS, runs the gate, spawns one
> subagent, updates STATUS, reschedules. All feature work happens inside subagents.

---

## AGENT CHARTER (paste into every spawned subagent, substitute <WS-xx>)

```
You are implementing <WS-xx> for the rubix edge-to-cloud data platform backend, as one autonomous
session in an unattended build. You run to completion and return — you cannot ask the human anything.

READ FIRST, IN ORDER:
1. /home/user/code/rust/awaken/CLAUDE.md             (the coding standard — governs every file)
2. rubix/docs/FILE-LAYOUT.md                         (file layout — one verb per file, ≤400 lines)
3. rubix/STACK-DEISGN.md                             (the platform target — crate map + contracts)
4. rubix/docs/SCOPE.md                               (scope authority — the why)
5. rubix/STATUS.md                                   (the gap inventory — your gap lives here)
6. rubix/docs/sessions/<WS-xx>.md                    (your spec — source of truth for scope)
7. rubix/docs/sessions/STATUS.md                     (what's already done — your deps are committed)

CODING STANDARD (CLAUDE.md is load-bearing — re-read it; the rules that bite hardest):
- Production-ready only. NO placeholder impls, NO `todo!()`/`unimplemented!()` in shipped paths,
  NO stubs that pretend to work, NO fallbacks that hide failures. Blocked? Log a TODO (see below).
- ONE RESPONSIBILITY PER FILE. No source file exceeds 400 lines (hook-enforced). Verb-per-file
  folders (create.rs/update.rs/run.rs), not one noun-file-does-everything. No utils.rs/helpers.rs/
  common.rs/misc.rs/types.rs/models.rs — name the concept. mod.rs is a barrel only.
- Search the repo FIRST for related/similar code; reuse or refactor to dedupe before adding new code.
- Error handling: thiserror + the project error enum; `.context()` for chaining. unsafe forbidden.
- Comments explain WHY not WHAT. No progress markers (// STAGE-1, // FIXED:, // Phase 1), no emoji
  in code. Bare TODOs forbidden — use `// TODO(loop):`. Code comments reference docs/ only, never
  these session docs.
- Honor docs/ specs (SCOPE.md + STACK-DEISGN.md contracts). If a change needs a contract exception,
  that's a TODO blocker — do not bypass a contract in code.

HARD RULES (this is an unattended run — violating these poisons every later session):
- BRANCH: work on `new-rubix`. Do NOT create branches or worktrees. Do NOT switch branches.
  Another AI session may be editing the same branch — commit only YOUR hunks (`git add -p` the
  files your WS owns), never blind `git add -A`, never revert changes you didn't make.
- NO QUESTIONS: you cannot prompt the human. If you hit a genuine ambiguity or need work a
  not-yet-run session owns, you DO NOT guess and DO NOT hack/stub. Instead:
    (a) append a dated entry to rubix/docs/sessions/TODOs.md in the documented format,
    (b) set your row in STATUS.md to ⛔ blocked with a one-line reason,
    (c) commit whatever compiles cleanly so far, then STOP and return a summary.
- NO HACKS: no `unwrap()`/`expect()` on fallible paths to "make it compile", no `#[ignore]` to dodge
  a failing test, no commented-out tests, no narrowing a test to pass. If you can't do it properly,
  it's a TODO entry, not a fake.
- STAY IN YOUR LANE: edit the files your WS owns. Touch a shared file (the workspace Cargo.toml,
  rubix-core's error enum, rubix-server's AppState/main, a barrel mod.rs) only as a minimal additive
  change. If a contract you depend on is missing, that's a TODOs.md blocker — do not redefine it.
- INCREMENTAL: implement one logical section, write its tests, run them green, commit, repeat
  (CLAUDE.md "Implementation Cycle"). Don't dump one giant commit.
- Ship mirrored tests (#[cfg(test)] units + integration tests in tests/). Keep
  `cd rubix && cargo test --workspace` AND `cargo clippy --workspace --all-targets` GREEN before you
  call yourself done. A red build or a clippy warning means you are NOT done.
- If you add/change env vars, DTOs, or schema, update STATUS.md's env list and (once WS-16 exists)
  the OpenAPI registration.

SESSION LOG (mandatory): create/maintain rubix/docs/sessions/<WS-xx>.md with:
  - a `Status:` line (In-progress / Blocked / Done) and a `Started:` + `Finished:` UTC timestamp,
  - the task breakdown you executed and what each commit did,
  - any assumptions, any deviations, any follow-ups.

FIRST ACTION (mandatory): confirm your dependency WSs are committed in the tree (read their
WS-xx.md `Done` lines + `git log`). If a dependency you need is missing, that's a TODOs.md blocker.
Then implement incrementally, commit (messages prefixed `<WS-xx>:` and following the commit
convention), and update STATUS.md + your session doc. When done, ensure the build/tests/clippy are
green and return a concise summary of what landed and what (if anything) you logged to TODOs.md.
```

---

## HEADLESS CRON MODE (the 100%-unattended path)

The loop survives a closed editor / sleeping session only when fired by the OS, not from a chat
window. The cron job runs **one wake per firing** with `claude -p` and exits — it is NOT the
in-session `/loop`. Each firing executes the LOOP ALGORITHM above exactly once.

**Concurrency lock (MANDATORY — prevents two firings double-spawning a WS):**
Before doing anything, the firing acquires an exclusive lock and skips if it can't:
```
exec 9>rubix/docs/sessions/.loop.lock
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

**The installer:** `sessions/install-cron.sh` writes the crontab line. To stop the run, the human
runs `./install-cron.sh remove` (or `crontab -r`). Kill switch without a crontab edit: a file
`sessions/.loop.STOP` makes every firing exit immediately without spawning.

## Notes for the loop driver
- **One subagent at a time.** Never spawn a second WS while one is 🔵 with a live subagent.
- **Timestamps:** the runtime has no clock inside scripts; when you (the loop) write timestamps,
  use `date -u` via Bash to get the real UTC time.
- **Crash recovery:** if the loop is restarted, step 1 reconstructs all state from STATUS.md +
  the per-session docs + `git log` — there is no hidden state. Safe to resume any time.
- **Definition of "all done":** every queue row is ✅, OR the remaining rows are ⛔ blocked and
  their TODOs are unresolved. Then STOP and report.
