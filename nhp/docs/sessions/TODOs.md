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
**Workaround (POC, no rubix change):** model enums as `text`, enforce the allowed set in the NHP
client layer (`nhp/collections/enforce.mjs`) + the UI dropdown. Not a blocker — WS-02 ships on the
workaround. **Correction (WS-02, 2026-06-16):** the `writeRule` route named here does NOT work —
verified that rubix's gate evaluates no `writeRule` (`rubix-core/src/collection/def.rs` keeps it as
raw JSON; `rubix-gate/src/command/validate.rs` enforces `required` + type only). Enums are enforced
client-side, not by a writeRule.
**Resolution:** _(rubix team — dated. Not required for the POC.)_

### 2026-06-16 — RUBIX-TEAM — Gate enforcement of `unique` and a per-collection write predicate
**Blocked on:** verified that the gate enforces only `required` + field TYPE. It does **not** enforce
`unique` (`rubix-core/src/collection/validate.rs`: "Uniqueness is not checked here") and evaluates
**no `writeRule`/access predicate** at all (`rubix-core/src/collection/def.rs` preserves `writeRule`
as raw JSON; the validate step never reads it). So NHP cannot enforce (a) unique keys or (b) the
per-network device limit (`network.max_devices` — reject an N+1th `meter`) at the gate.
**Needs:** rubix team to (1) realise `unique` via `DEFINE INDEX` on collection define
(BACKEND-COLLECTIONS open question 11) and (2) add a gate-evaluated `writeRule` predicate, generic
and reusable (BACKEND-COLLECTIONS open question 4). Both are upstream, approved by the rubix team —
**NHP does not implement them.**
**Workaround (POC, no rubix change):** enforce `unique` and the device limit in the NHP client layer
(`nhp/collections/enforce.mjs`) and the onboarding wizard (WS-06). Defence-in-depth becomes
defence-in-one until the gate enforces it; acceptable for the POC.
**Resolution:** _(rubix team — dated. Not required for the POC.)_

### 2026-06-16 — RUBIX-TEAM — HTTP route to attach tag-graph edges to a record
**Blocked on:** rubix tags are graph edges (`record→tagged→tag`) and the list read projects them onto
`RecordDto.tags`, but the HTTP records API exposes **no route to WRITE a tag edge**
(`rubix/crates/rubix-server/src/http/records/` has create/get/list/update/delete only; the seed
library attaches tags via the gate directly, which NHP cannot call). So an HTTP-only client (NHP)
cannot create true tag-graph edges.
**Needs:** rubix team to add a generic tag-attach/detach HTTP surface (e.g.
`PUT/DELETE /records/:id/tags/:tag`) routed through the gate, reusable by any record consumer —
upstream, approved by the rubix team. **NHP does not implement this.**
**Workaround (POC, no rubix change):** NHP carries its standard tags in the record's `content.tags`
array (written over the normal records API); WS-03's `seed/tags.mjs` is the single source of truth and
WS-07's auto-build reads `content.tags`. Functionally sufficient for the POC — the cost is that tag
filtering can't use rubix's tag-graph query, only content filtering. Not a blocker.
**Resolution:** _(rubix team — dated. Not required for the POC.)_
