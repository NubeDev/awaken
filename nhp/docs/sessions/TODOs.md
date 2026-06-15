# NHP POC — Blocker Log

When a workstream hits a genuine ambiguity, or needs a contract a not-yet-run session owns, or needs
work outside its lane (e.g. a rubix backend change beyond the OVERVIEW gaps), it does **not** guess
or hack. It appends an entry here, sets its STATUS row to ⛔, commits what works, and stops.

The **human** resolves a blocker by editing the entry: add a dated `Resolution:` line (and/or strike
the entry through). On the next wake the loop sees the dated Resolution, resets the ⛔ row to ⬜, and
re-queues it. An entry with no dated Resolution stays blocking — the loop never self-unblocks.

## Format

```
### <UTC timestamp> — WS-xx — <one-line title>
**Blocked on:** what's missing / the ambiguity, concretely.
**Needs:** the decision or the dependency that would unblock it.
**Workaround considered:** what was tried / why it's not acceptable as a POC shortcut.
**Resolution:** _(human fills this — dated. Until then the row stays ⛔.)_
```

## Entries

_(none yet)_
