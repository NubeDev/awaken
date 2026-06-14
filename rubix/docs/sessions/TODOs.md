# Rubix Build — Blocker / Follow-up Log

Append-only log of blockers and deferred follow-ups raised by unattended workstream sessions. A
session that cannot proceed properly (genuine ambiguity, a missing dependency a not-yet-run WS owns,
or a contract exception that would otherwise require a hack) logs here, sets its STATUS.md row to ⛔
blocked, commits what compiles, and stops.

Format (one entry per blocker):

```
## <UTC timestamp> — WS-xx — <one-line title>
- **What:** what is blocked and where (file:line if known).
- **Why:** why it can't be done properly now (the dependency / ambiguity / contract conflict).
- **Needs:** what unblocks it (which WS, which human decision).
- **Resolution:** (filled when resolved — struck through or dated. The loop resets the row to ⬜
  once this is resolved.)
```

When the human resolves a blocker, strike the entry through or add a `Resolution:` line; the loop
then resets that WS's row to ⬜ and re-picks it in queue order.

---

## 2026-06-15T06:15:00Z — WS-13/14/15 — Deferred (edge/extensions/sync infrastructure)

- **What:** WS-13 (extensions), WS-14 (edge/cloud profiles), WS-15 (sync shipper) marked blocked.
- **Why:** User focus is on core backend transport (WS-16) first; edge/extension/sync infrastructure deferred.
- **Needs:** User decision when to implement. For now, WS-12 → WS-16 directly (skip 13/14/15).
- **Resolution:** Unblock when user is ready to ship edge/extensions features.
