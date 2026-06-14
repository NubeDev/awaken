# Fix Loop — The AI Working Contract

> You've captured evidence ([CAPTURE.md](CAPTURE.md)) and identified a root cause
> ([TRIAGE.md](TRIAGE.md)). This is how to change rubix and **prove** the fix.

---

## Before you change anything

1. **State the root cause in one sentence**, citing the file:line and the evidence
   artifact that proves it. If you can't, you're guessing — go back to triage.
2. **Decide: rubix bug, test/doc bug, or expected behavior?**
   - Test/doc bug → fix the doc, bump `Verified:`, done. Don't touch rubix.
   - Expected behavior (escalation suspend, owned-site silence, unscoped=global) →
     record it in the feature doc's "Gotchas", done.
   - rubix bug → proceed.
3. **Find the smallest change** that addresses the cause, not the symptom. A
   symptom patch (e.g. widening a type to swallow bad data) usually moves the bug.

---

## Making the change

- This is a Rust workspace (`rubix/crates/rubix-*`). Match surrounding idioms:
  `thiserror` error enums, `.context()` chaining, `#[cfg(test)]` modules. Honor the
  hard rules in `rubix/CLAUDE.md` (no temp solutions, no placeholder impls,
  production-ready only) and the awaken `CLAUDE.md` architecture guardrails.
- `unsafe_code = "forbid"` is a workspace lint — never reach for it.
- **File-layout discipline**: no source file exceeds 400 lines (STATUS.md invariant).
  If a fix would push a file over, split it.
- One logical fix per change. If you find a second bug, capture it separately.
- If the fix touches an HTTP DTO, it must stay in sync with the utoipa
  `#[utoipa::path]` annotation so `/api-docs/openapi.json` reflects it.
- If it touches the SQLite schema, follow the store's schema-apply path
  (`SCHEMA_SQLITE`) — don't hand-edit a live DB.

---

## Proving the fix (mandatory)

A fix is not done until **all** hold:

1. ✅ `make test` green (and `make build`, `make lint` clean — `make lint` runs
   `cargo clippy --all-targets -- -D warnings`; the workspace ships clippy-clean per
   STATUS.md). Backend-only: `make test-be` / `make build-be`.
2. ✅ The originally-failing ✅ check now passes — re-run that exact step.
3. ✅ The full scenario it belonged to is green end-to-end (no new red). See
   [../scenarios/README.md](../scenarios/README.md).
4. ✅ A fresh evidence bundle in a new timestamp dir shows the symptom gone
   (before/after comparable — `db_state.txt` and `query_count.json` now agree, the
   `cur_value` lands, the run commits, whatever the symptom was).
5. ✅ No silent regression: the log/metric line that was wrong is now right *for the
   right reason* (you can explain it), not just absent.

---

## Record it

In the relevant `features/<X>.md` "Known issues / fixes" section, append:

```md
### <date> — <one-line symptom>
- **Symptom:** <expected vs actual>
- **Evidence:** testing/.evidence/<scenario>/<ts>/
- **Root cause:** <file:line + why>
- **Fix:** <what changed> (commit <hash>)
- **Verified:** re-ran <step/scenario> → green; before/after bundle <ts2>
```

This turns each fix into institutional memory the next session can search.

---

## When to stop and ask

- The fix requires a design decision (new HTTP DTO shape, new table, changing the
  priority-gating or scope model) → surface it, don't unilaterally reshape
  contracts. The priority array, capability `covers`, and escalation bands are
  load-bearing safety invariants.
- The "bug" is actually a missing feature (e.g. hot scheduler reconfiguration,
  zenoh board deploy, the cross-mesh `points_cur` variant — all listed "Remaining"
  in STATUS.md) → that's a feature task, not a fix; note it and pick it up
  deliberately.
- The cause is in a crates.io dependency rubix pins (`reflow_*` 0.2, `awaken-runtime`
  0.6, `zenoh` 1, `datafusion` 53) → rubix has **no fork**; flag the blast radius
  before working around it, don't vendor a patch silently.

---

## Optional: scripted driver

For repeated regression runs, a `testing/scripts/run-scenario.sh <name>` that boots
the stack, runs a scenario's gates, and auto-captures on first failure makes the
loop one command. Build it when the manual loop stabilizes; reference it from
[../scenarios/README.md](../scenarios/README.md). Keep the human/AI in the loop at
the fix step — autonomous code changes still get the full "proving the fix" gate.
