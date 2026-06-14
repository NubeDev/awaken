# Audit Log & Undo/Redo — one change ledger, for everything

Scope for a single append-only **change ledger** that powers two product surfaces
at once: an **audit log** (who changed what, when, before→after) and **undo/redo**
(replay a change in reverse or forward). The ask is undo/redo *and* audit, *for
everything* — sites, equips, points, dashboards, widgets, boards, rules, users,
teams, grants, datasources — built the way that holds up long term.

The long-term answer is **not two systems**. Audit and undo are two reads of one
substrate: every domain mutation appends one immutable row; the audit log *queries*
those rows, undo/redo *replays* them. Adding a new audited/undoable entity becomes
"register one reverser + emit one row after the mutation" — never a new feature.

This is greenfield. Rubix has no changelog, no mutation history, and no undo today
(verified: `store/schema.rs` has no audit/change table; the only "history" is the
`his` time-series of point *values*, which is unrelated). There is no
`starter-changelog`/`starter-undo` dependency to adopt — unlike the nexus design
this borrows from, rubix builds the substrate itself, kept deliberately small.

## Problem

Today every mutation is destructive and unrecorded:

- Renaming a site, re-tagging an equip, deleting a dashboard, editing a rule — none
  leave a trace. There is no "who deleted this point" and no way to put it back. A
  cascade delete of a site (every equip, point, history sample, spark beneath it)
  is irreversible.
- An admin auditing a tenant has nothing to read. There is no record of grant
  changes, no record of who provisioned an org, no before/after on a rule edit.
- The agent runtime (`crates/rubix-server/src/agent/`) mutates entities on a user's
  behalf with **no record and no undo** — the weakest safety story in the product.

The two needs share one shape: an immutable, ordered, attributed list of
"X changed from A to B at time T by actor P". Build that once.

## The substrate

### The change row

One table, `changes`, append-only, one row per logical mutation:

```
Change {
  id:          Uuid,             // change id, monotonic by (at, id)
  at:          DateTime<Utc>,    // when committed
  org:         String,           // tenant key — every read is org-filtered
  site_id:     Option<Uuid>,     // site scope when the entity is site-scoped
  actor:       Actor,            // who: User | Agent | System
  kind:        String,           // resource kind: "dashboard", "point", "rule", …
  resource_id: Uuid,             // the mutated row's id
  op:          Op,               // Create | Update | Delete
  before:      Option<Value>,    // full snapshot before (None for Create)
  after:       Option<Value>,    // full snapshot after  (None for Delete)
  group_id:    Uuid,             // groups multi-row mutations into one undo step
  correlation: Option<String>,   // request id / agent run id for tracing
}

Actor = User { subject: String }                 // RBAC subject (reuses auth identity)
      | Agent { run_id: Uuid, model: String }    // the agent runtime — same ledger
      | System                                   // scheduler / provisioning
```

Design choices that matter long term:

- **Snapshot, not patch.** `before`/`after` are full JSON snapshots of the row, not
  diffs. Snapshots make undo trivial (write the snapshot back), make "restore to
  this point" possible without walking a patch chain, and keep each row
  self-contained. Rows are small (a dashboard/widget/point is a few hundred bytes);
  if a future entity's snapshot is genuinely large, redact non-essential fields at
  record time rather than switching to patches.
- **`Actor::Agent` is the AI-agent log.** Agent edits land in the same ledger with
  `run_id` + `model`, so the agent audit trail and user audit trail are one query,
  and **agent edits are undoable by the user** — the safety story for AI-driven
  changes (`crates/rubix-server/src/agent/`).
- **`group_id` groups a transaction.** Add-widget-and-update-layout is two rows, one
  `group_id`, undone as one step. The recorder assigns one `group_id` per logical
  operation by default.
- **`org`-scoped, always.** Every read filters by `org` (the rubix tenant key,
  `docs/design/crud-and-tenancy.md`); cross-tenant audit reads are impossible by
  construction, matching the existing two-layer authz model
  (`docs/design/authz-rbac.md`).

### The reverser (the one extension point)

Per resource kind, one small impl says how to undo/redo it:

```
trait Reversible {
  fn kind(&self) -> &'static str;
  fn apply_inverse(&self, store: &Store, c: &Change) -> Result<()>;  // undo
  fn apply_forward(&self, store: &Store, c: &Change) -> Result<()>;  // redo
}
```

Because everything is a snapshot, the default impl is mechanical:

- **undo Create** → delete `resource_id`.
- **undo Delete** → re-insert `before`.
- **undo Update** → write `before` back.
- **redo** → the forward of each (insert `after`, delete, write `after`).

A generic snapshot reverser covers most kinds by deserializing `before`/`after`
into the kind's model and calling the existing store create/update/delete. A kind
only needs custom code when undo has a side effect (e.g. undeleting a site must
*not* resurrect cascade-deleted children unless they were captured in the same
`group_id` — see Cascade below).

### Schema & migration

A new migration bumps `PRAGMA user_version` (forward-only, the existing pattern in
`crates/rubix-server/src/store/migrate.rs`, currently at v4):

- `changes` table (columns above), indexed on `(org, at)`, `(org, kind, resource_id)`,
  and `group_id`. Mirror the dual-dialect shape in `store/schema.rs` (SQLite base +
  `#[cfg(feature = "cloud")]` Postgres).
- `undo_cursors` table: `(org, subject)` → current cursor position, so undo/redo is
  **per-actor** (undoing your change doesn't undo a colleague's). A version/epoch
  column guards concurrent undo (compare-and-set on pop).

## Recording — the "for everything" part

A `ChangeRecorder` (new, in `store/changes.rs`) is the only write path. The pattern,
dropped into each mutation handler right after the successful store call, inside the
same DB transaction so the change commits atomically with the mutation:

```rust
// in api/dashboards/patch.rs (illustrative)
let before = store.get_dashboard(org, id)?;          // already read for authz
let after  = store.update_dashboard(org, id, patch)?;
recorder.record(Change::update(
    Actor::User { subject: principal.subject() },
    "dashboard", id, org, site_id,
    json(&before), json(&after),
))?;
```

The handler already holds `before` (it reads the row for authz/existence) and
`after` (what it wrote), so recording adds one call, no extra fetch.

**Kinds to cover** (the user's "users, dashboards, datasources and so on"), each
wired in its own handler so the change is recorded next to the mutation it
describes:

| Kind | Handlers | Notes |
| --- | --- | --- |
| site | `api/sites/{create,patch,delete}.rs` | delete is a cascade — see Cascade |
| equip | `api/equips/{create,patch,delete}.rs` | |
| point | `api/points/{create,patch,delete}.rs` | value writes (`cur`/`write`) are **not** recorded here — they flow to `his`, not the audit ledger |
| dashboard | `api/dashboards/{create,patch,delete}.rs` | cascade to widgets |
| widget | `api/widgets/{create,patch,delete}.rs` | |
| board | `api/boards/{create,patch,delete}.rs` | |
| rule | `api/rules/{create,update,delete}.rs` | |
| datasource | `api/datasources/**` | **redact secrets** (connection strings/tokens) before snapshot — see below |
| user / team | `api/users/`, `api/teams/` | provisioned via OIDC; record what's mutated |
| grant | `api/grants/` | grant/revoke is security-sensitive — audit is the point |
| token | `api/tokens/` | record create/revoke; **never snapshot the secret** |
| org | `api/orgs/create.rs` | provisioning, `Actor::System` or the provisioning admin |

> **Secret redaction is a recording contract.** `before`/`after` must never capture
> a plaintext secret — a datasource connection password, a PAT, an agent API key.
> The recorder for a secret-bearing kind redacts those fields at record time
> (replace with a `"***"` sentinel), so the audit log shows "connection changed",
> not the value. Owners of secret-bearing kinds assert this in tests.

Read-only verbs record nothing. Value writes to points (`api/command/`) are
high-frequency telemetry, not configuration changes — they stay in `his` and are
out of scope for the audit ledger.

### Cascade deletes

A site delete cascades to equips/points/history/sparks (`store/sites.rs`). For undo
to restore the whole subtree, the cascade must record **one row per deleted child
under a shared `group_id`** (the parent's), so undo replays them as one step. The
site delete handler captures each child snapshot before the cascade and records the
group. This is the one place the generic reverser needs the owning handler's help.

### Coverage guard

The worst failure mode for an audit log is being *silently partial* — it looks
complete but a kind quietly isn't recorded. A test enumerates the registered
reversible kinds and asserts each has at least one create/update/delete path that
produces a `changes` row, and that `before` is non-null on update (catches the easy
mistake of recording outside the transaction so the pre-read returns nothing). A
registered mutable kind with no recording path **fails the test**.

## Audit read surface

New routes over the ledger, org-scoped via the existing authz layer:

- `GET /api/v1/audit` — filter by `kind`, `resource_id`, `actor`, `op`, time range;
  paged, newest-first; returns `Change` rows with `before`/`after` for diff
  rendering.
- `GET /api/v1/audit/{kind}/{id}` — the history timeline for one resource (powers a
  "History" tab on a dashboard, datasource, rule, …).

**Authz:** audit read is privileged — gated behind an admin/`audit:read`
capability in the role model (`docs/design/authz-rbac.md`). A tenant admin sees
their org's log; cross-org reads require platform super-admin. Never leak another
org's rows (org filter + grant check, like every other read).

DTOs derive `ToSchema` and register in `api/openapi.rs`; the hand-written TS types
in `ui/src/api/types.ts` gain `Change`/`Actor`/`Op` mirrors (rubix has no codegen —
types are authored by hand, confirmed).

## Undo/Redo

- `POST /api/v1/undo` / `POST /api/v1/redo` — target the authenticated principal,
  pop/advance their cursor, dispatch the group through the reverser registry, and
  return the affected `group_id` + touched resource ids so the UI can invalidate
  exactly those queries.
- Per-actor cursor (`undo_cursors`), CAS-guarded so two concurrent undos don't
  double-pop. Undo targets *your* changes only — cross-actor global undo is a
  non-goal (it makes "undo" unpredictable in a multi-user tenant).

## UI

The frontend stack is React 19 + TanStack Query + TanStack Router + zustand
(`ui/package.json`), with the dashboard builder at `ui/src/features/builder/`.

- **Undo/Redo control + shortcuts.** Cmd/Ctrl+Z / Shift+Cmd/Ctrl+Z and a toolbar
  button (mount in `ui/src/components/layout/page-header.tsx`, currently bare).
  On success, use the returned touched-resource ids to invalidate the matching
  TanStack keys (e.g. `qk.widgets(...)`, `ui/src/api/keys.ts`) so the canvas
  refreshes without a reload.
- **Toast with inline undo** after a mutation ("Deleted dashboard · Undo"),
  carrying the `group_id`.
- **Audit/History UI:** a per-resource "History" tab (timeline + before→after diff)
  and an admin "Audit log" screen with the filters above, gated behind the audit
  capability.

## What to prove

1. Editing a dashboard appends one `changes` row (actor, before/after, inside the
   tx); `POST /undo` restores it and refreshes the canvas; `POST /redo` re-applies.
2. A multi-row action (add-widget + layout patch; a site cascade delete) undoes as
   one `group_id`.
3. The same flow works unchanged for a datasource and a rule — proving "for
   everything" rides the registry, not per-feature code.
4. `GET /audit` returns who/what/when with before→after, filterable, **org-isolated**
   (a cross-org read is impossible), gated behind the audit capability.
5. An agent-made edit records with `Actor::Agent{run_id,model}` and is undoable by
   the user.
6. Secret-bearing snapshots are redacted (a datasource password never appears in a
   `changes` row).
7. The coverage guard is green; deliberately unwiring a kind makes it fail.

## Acceptance criteria

- [ ] `changes` + `undo_cursors` tables land via a forward-only `user_version` bump,
      dual-dialect (SQLite + Postgres cloud feature), org-scoped.
- [ ] `ChangeRecorder` is the sole write path; every kind in the table above records
      create/update/delete inside the mutation's transaction.
- [ ] `Reversible` registry with a generic snapshot reverser; custom impls only where
      a cascade or side effect needs it.
- [ ] Cascade delete records one group; undo restores the whole subtree atomically.
- [ ] `POST /undo` / `POST /redo` are per-actor, CAS-guarded, and return touched ids.
- [ ] `GET /audit` + `GET /audit/{kind}/{id}` are org-isolated and capability-gated.
- [ ] Secret redaction enforced at record time for datasources/tokens/agent keys.
- [ ] Coverage guard fails on an unwired registered kind.
- [ ] UI: undo/redo shortcuts + toast + query invalidation; per-resource History tab;
      admin audit screen.
- [ ] Tests: record-on-mutate per kind, undo/redo round-trip + grouping, cascade undo,
      audit query + org isolation, agent-actor undo, secret redaction, coverage guard.

## Out of scope (hand off)

- Point *value* history (`his`) — already exists, unrelated to config audit.
- Cross-actor "global undo" — explicit non-goal.
- Full identity (user/team) undo beyond snapshot restore — audit first; undo of
  identity mutations is a fast-follow once the auth handlers adopt the recorder.
- Variable/context recording — covered as part of the dashboard kind; see
  [variables-and-templating.md](variables-and-templating.md) and
  [page-context-and-nav.md](page-context-and-nav.md).
