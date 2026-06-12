# Rubix Build — Blockers & Follow-ups

Append-only log of things an unattended session could NOT do properly and refused to hack. The human
resolves an entry, then strikes it through (`~~...~~`) or deletes it; the loop resets the
corresponding ⛔ row to ⬜ on its next wake.

## Format

```
### <utc-date> — <WS-xx> — <one-line title>
- **What's blocked:** ...
- **Why (the ambiguity / missing dep / guardrail conflict):** ...
- **What the human must decide/provide:** ...
- **Committed so far:** <commit sha or "nothing — clean working tree">
```

---

### 2026-06-12 — (pre-existing, surfaced by WS-01) — `chat_reports_awaiting_approval_when_a_write_suspends` hangs
- **What's blocked:** A clean `cargo test --workspace` run does not terminate — the test
  `api_tests::agent::chat_reports_awaiting_approval_when_a_write_suspends`
  (`rubix-server/tests/api_tests/agent.rs`) runs past 200s and is killed by timeout.
- **Why:** Confirmed pre-existing, NOT caused by WS-01: reproduced on a clean HEAD with all WS-01
  changes stashed (source + tests), the test still hangs >200s. It exercises the HITL escalation-band
  suspend path (`write_point` at priority 5 → suspended run → `awaiting_approval`); the scripted-agent
  suspend path in awaken-runtime 0.6 appears to block. WS-01 touches none of this path (only the
  `agent_call` board node, `request_agent_blocking`, and `BoardGraph::run` settle loop), and all WS-01
  tests plus `rubix-server` lib/`agent_call` integration tests pass.
- **What the human must decide/provide:** whether the suspend/await-approval run path has a missing
  resume/terminate signal (likely lands with WS-02's persistent run store), or the test needs a bound.
- **Committed so far:** WS-01 work committed on `rubix-gaps`; this entry is a follow-up, not a WS-01
  blocker (WS-01 is Done for its own scope and green).
