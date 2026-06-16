# NHP POC — Blocker Log

When a workstream hits a genuine ambiguity, or needs a contract a not-yet-run session owns, or needs
Rust that NHP isn't allowed to write (see "Rubix is frozen" below), it does **not** guess or hack.
It appends an entry here, sets its STATUS row to ⛔ (if there's no acceptable POC workaround),
commits what works, and stops.

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

## Rubix is frozen — `RUBIX-TEAM:` entries

NHP **never edits rubix source.** When a WS finds it needs Rust:
- **Generic & reusable** (a core capability others would use) → a `RUBIX-TEAM` entry below. The
  rubix team implements it with their approval; NHP takes the data/UI workaround meanwhile. Only set
  the WS row ⛔ if there's no acceptable POC workaround.
- **NHP-specific Rust** → a rubix **extension** (`rubix-ext`, see
  [rubix/crates/rubix-ext/README.md](../../../rubix/crates/rubix-ext/README.md)) — only if a WS spec
  authorises building one; otherwise log it here.

## Entries

### 2026-06-16 — RUBIX-TEAM — Native `Select`/enum field type for collections
**Blocked on:** rubix `FieldType` has no closed-enum/`Select` variant; NHP's closed enums
(`net_type` 485/ethernet, `protocol`, register `datatype`/`fn_code`/`byte_order`, `chart_type`) have
no native typed enforcement.
**Needs:** rubix team to add a generic `Select { options }` variant to
`rubix/crates/rubix-core/src/collection/field.rs` + its validation (BACKEND-COLLECTIONS open
question 3). Generic + reusable by any collection consumer — an upstream addition, approved by the
rubix team. **NHP does not implement this.**
**Workaround (POC, no rubix change):** model enums as `text`, enforce the allowed set via the
collection `writeRule` + the UI dropdown. Not a blocker — WS-02 ships on the workaround.
**Resolution:** _(rubix team — dated. Not required for the POC.)_
