# Rubix UI Build — Workstream Queue (finish the UI, zero fake data)

Companion queue to the backend queue in [../STATUS.md](../STATUS.md), driven by the same loop
algorithm in [../_ORCHESTRATION.md](../_ORCHESTRATION.md). Branch: **`rubix-gaps`**, sequential,
no worktrees.

## Mission

`rubix/ui` looks right (the operator has signed off on the look) but runs on an in-memory demo
dataset (`src/api/demo/`). The mission is to make every surface read and write the **real**
rubix-server API with **zero fake data in the UI**, without regressing the look.

**The look is frozen.** The current demo-mode rendering is the visual contract: density, dark
theme, KPI sparklines, the labelled priority array, the sparks master-detail, the flows chrome.
"Remove fake data" means *change where data comes from*, never *what populated data looks like*.
The thing that preserves the look with real data is **UI-02** (server-side dev seed): the demo
building moves from TypeScript fixtures into the real store, served by real endpoints.

What counts as fake (the removal targets):
- `ui/src/api/demo/fixtures.ts` + `ui/src/api/demo/index.ts` and the `isDemo()` switch in
  `ui/src/api/endpoints.ts` (defaults ON today).
- `ui/src/features/flows/data/sample-board.ts` (static wiresheet graph).
- Hardcoded UI claims: `board-status-bar.tsx` ("deployed · running", "v4"),
  `sparks/index.tsx` (`agentAttributed={s.rule === 'simultaneous-heat-cool'}`),
  `profile-dropdown.tsx` ("satnaing").
- TS wire types that were *assumed* rather than verified against the server
  (`RunSummary`, `QueryResult`, `PriorityArray` JSON shape, `WriteSource` casing).

What does NOT count as fake (allowed to stay):
- Honest empty/loading/error states when the store genuinely has no data.
- The dev seed (UI-02): real rows in the real store via real endpoints, dev-gated.
- Test fixtures inside `*.test.ts` files (the operator explicitly allows test-only data).

## Queue

| # | Workstream | Status | Started | Finished | Commit |
| --- | --- | --- | --- | --- | --- |
| UI-01 | Wire-contract truth-up: TS types verified against OpenAPI | ✅ | 2026-06-12T22:29:06Z | 2026-06-12T22:39:09Z | f3af3bc5 |
| UI-02 | Dev seed: the demo building as real store rows + live sim | ✅ | 2026-06-12T22:45:13Z | 2026-06-13T01:10:00Z | d5c349f8 |
| UI-03 | Delete the demo layer; UI reads the network only | ✅ | 2026-06-12T23:05:15Z | 2026-06-12T23:12:00Z | 3bf7f5b1 |
| UI-04 | Flows on stored boards (`/api/v1/boards`) | ✅ | 2026-06-12T23:15:14Z | 2026-06-13T06:25:00Z | 5c28d145 |
| UI-05 | Agent surface: runs, resume/cancel, diagnose, HITL UX | ⬜ | | | |
| UI-06 | Dashboard Builder MVP on `/api/v1/widgets` | ⬜ | | | |
| UI-07 | Cleanup & hardening: template leftovers, auth header, tests, lint | ⬜ | | | |

Queue order is dependency order. UI-01/UI-02 are prerequisites of UI-03 — **the demo layer must
not be deleted before the seed exists**, or every screenshot regresses to empty states and the
look-freeze gate cannot pass.

## DONE GATE (UI variant — replaces the cargo gate for these rows)

Before marking any UI-xx ✅:
- `pnpm -C rubix/ui build` green (tsc -b + vite build).
- `pnpm -C rubix/ui test:unit` green.
- **Grep gate** (from UI-03 onward):
  `grep -rn "api/demo\|VITE_DEMO\|sample-board" rubix/ui/src --include='*.ts*'` → no hits
  outside `*.test.*`.
- **Look-freeze gate**: with `rubix-server` + seed running, capture
  `scripts/screenshot.mjs` shots of dashboard / points / sparks / flows and eyeball against the
  reference set in `rubix/ui/docs/reference/` (captured by UI-02 before any demo removal).
  A populated page may not lose widgets, density, or chart series vs its reference.
- Backend untouched unless the WS explicitly owns a backend change (only UI-02 does);
  if touched: `cd rubix && cargo test --workspace` + clippy clean.
- Session wrote `Done` + timestamp in its `ui/UI-xx.md`; commit on `rubix-gaps` prefixed `UI-xx:`.

## Charter deltas (in addition to ../\_ORCHESTRATION.md AGENT CHARTER)

- READ FIRST also includes: `rubix/ui/src/api/` (the client), the OpenAPI doc served at
  `/api-docs/openapi.json` (run the server; it is the wire truth), and
  `/home/user/code/rust/starter/rubix/FILE-LAYOUT.md` (file discipline applies to TS too:
  ≤400 lines hard, verb/concept per file, no utils.ts).
- The UI dev loop: `cd rubix && cargo run -p rubix-server` (port 8088) then
  `pnpm -C rubix/ui dev` (port 5180, `/api` proxied). `CHOKIDAR_USEPOLLING=true` if inotify
  limits bite.
- shadcn/ui components in `src/components/ui/` are vendored library code — extend variants,
  do not rewrite.
- Never invent a wire field. If the UI needs data the API doesn't expose, that is a TODOs.md
  blocker entry (`../TODOs.md`), not a synthesized value.

## Dependency notes

- **UI-01 → everything**: all later rows assume the client types are the verified wire truth.
- **UI-02 → UI-03**: seed first, delete second (look-freeze depends on it). UI-02 is the only
  row allowed backend changes (a seed path + sim wiring) and must pass the cargo gate too.
- **UI-04/05/06** are independent of each other once UI-03 lands; keep queue order anyway
  (sequential branch discipline).
- **UI-07** last: it deletes/locks things earlier rows still touch.

## Loop log
<!-- one line per wake: <utc> <action> -->
- (pending first wake)
- 2026-06-12T22:15:41Z skip — UI-01 still 🔵, subagent live (uncommitted edits across types.ts/hooks.ts/endpoints.ts/tags.ts + history/runs, ~6 min in, no Done/Blocked line); no double-spawn, reschedule
- 2026-06-12T22:26:00Z skip — UI-01 still 🔵, subagent live (working tree mutating mid-wake: types/hooks/endpoints/tags.ts reverted-clean between two `git status` calls, history/index.tsx still M, no new commits, UI-01.md still Pending); no double-spawn, reschedule
- 2026-06-12T22:29:06Z spawn — UI-01: prior subagent dead (row was ⬜/UI-01.md Pending, only leftover M history/index.tsx which UI-01 owns, no new commits since); first pending row → spawn fresh UI-01 subagent (idempotent resume from committed state)
- 2026-06-12T22:40:54Z gate ✅ — UI-01: subagent returned Done (UI-01.md Status: Done, Finished 22:39:09Z); DONE GATE green (pnpm build ✓, test:unit 18/18 ✓, commit f3af3bc5 `UI-01:`-prefixed, backend untouched); row → ✅. Next pending: UI-02.
- 2026-06-12T22:45:13Z spawn — UI-02: no WS 🔵, UI-01 ✅, first pending row → spawn fresh UI-02 subagent (dev seed; only UI row allowed backend changes, must pass cargo + UI gate).
- 2026-06-13T01:10:00Z gate ✅ — UI-02: subagent returned Done; DONE GATE green (pnpm build ✓, test:unit 18/18 ✓, cargo test --workspace ✓, clippy clean ✓, 4 reference screenshots in ui/docs/reference/ ✓; commits `UI-02:`-prefixed, head d5c349f8); row → ✅. Next pending: UI-03.
- 2026-06-12T23:05:15Z spawn — UI-03: no WS 🔵, UI-01/UI-02 ✅, first pending row → spawn fresh UI-03 subagent (delete demo layer; grep gate goes live).
- 2026-06-12T23:12:57Z gate ✅ — UI-03: subagent returned Done (UI-03.md Status: Done); DONE GATE green (pnpm build ✓, test:unit 18/18 ✓, grep gate ✓ — only `sample-board` hit is flows/index.tsx, owned by UI-04 & excluded per spec exception; look-freeze verified by subagent; commits `UI-03:`-prefixed, head ce45ebfd); row → ✅. Next pending: UI-04.
- 2026-06-12T23:15:14Z spawn — UI-04: no WS 🔵, UI-01/02/03 ✅, first pending row → spawn fresh UI-04 subagent (flows on stored `/api/v1/boards`; delete sample-board.ts; grep gate now covers it).
