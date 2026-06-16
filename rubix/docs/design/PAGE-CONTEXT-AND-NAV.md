# PAGE-CONTEXT-AND-NAV — reuse one board across a fleet via a nav tree

Feature scope for making **one board serve a whole fleet** by separating the *page*
(a parameterised board) from the *place* it is viewed for (a site, a building, a
level). It builds directly on [VARIABLES-AND-TEMPLATING.md](./VARIABLES-AND-TEMPLATING.md)
(build that first — context is a new *source* that feeds the same variables) and
re-grounds the old-rubix `page-context-and-nav.md` in this **records-backed** rubix.

Three coupled pieces:

1. **Page context** — a board reads the **URL**, its **nav-tree position**, and its
   **tags**, and feeds those into variables. One `energy-overview` board under
   *Building-1* queries Building-1's data; the same board under *Building-2* queries
   Building-2's — no second board authored.
2. **A navigation tree** — a nested, user-built nav where each node **mounts a board
   (or a static route) and carries a context payload**. Click a node → open its board
   bound to its context.
3. **Nav-based access** *(see the departure note — partial in this architecture)*.

> **rubix-here vs old-rubix.** Three deliberate departures, because the platform
> differs (`docs/SCOPE.md`, `STACK-DEISGN.md`):
> - **No relational tables / migrations.** A nav node is a `kind:"nav_node"` **record**
>   (free-form `content`, like a board or chart), created through the generic
>   `POST /records` surface under `IngestPublish`. There is **no `nav_nodes` table and
>   no migration** — the records-backed store is the model. The UI owns the node shape
>   exactly as it owns the board shape (`ui/src/api/boards.ts`).
> - **Tenancy is the namespace, not org+RLS.** A record carries a `namespace`; the
>   scoped session enforces `WHERE namespace = $auth.namespace` natively. Nav nodes are
>   namespace-scoped automatically — no org column, no RLS policy to author.
> - **Access is coarse, not per-node.** rubix's authz is two layers: SurrealDB
>   namespace scoping + **namespace-level capability grants** (`rubix-gate`,
>   `Capability`). There is **no per-record grant/sharing** primitive. So the old
>   doc's headline "grant `view` per nav node" does **not** map: every principal in a
>   namespace sees every node in it. Per-node visibility is a **documented follow-up**
>   (it needs a new per-record ACL subsystem — a `principal →can_view→ record` edge
>   wired into the gate and a read-filter), explicitly **out of scope** here. The
>   `route`-kind allow-list and the default tree still ship, so the *structure* is
>   ready for ACLs when that subsystem lands.

## Problem / current state

- **Navigation is a flat, static list.** `ui/src/components/shell/nav.ts` is a
  hard-coded `NAV: NavGroup[]` of route links — no nesting, no user-built nodes, no
  per-node context.
- **"Context" today is the route param only.** A board opens by id; nothing reads a
  nav node, bare URL params, or tags to parameterise it.
- **No variables to feed** — added by [VARIABLES-AND-TEMPLATING.md](./VARIABLES-AND-TEMPLATING.md).
  Context is a new *source* for that engine, not a parallel one.
- **Tags exist but classify, not navigate.** `rubix-core::tag` is a global graph for
  domain classification (site/equip/point), namespace-scoped via the `tagged` edge.
  Dashboard/board tags are a new, behaviour-affecting use.

## Scope

### 1. Page context model (UI)

A board resolves against a `PageContext` assembled at view time from sources kept
**separate** (not pre-flattened), so a `context` variable addresses exactly one and
precedence is explicit:

```
PageContext {
  nav?:   { node_id, title, path: string[] },   // the node the board opened under
  url:    Record<string, string | string[]>,    // bare query params (not var-*)
  tags:   Record<string, string | null>,        // the board's tags
  values: Record<string, string | string[]>,    // node.context.values overrides
}
```

`NavNode.context` is exactly `{ values?, tags? }`:

- `context.values` → `PageContext.values` (a node `{ values: { site: "hq" } }` makes
  `$site` resolvable). The override path.
- `context.tags` → merged **over** the board's own tags into `PageContext.tags`.

**Precedence (later wins), per variable name:** board tags → `nav.values` → URL params
→ explicit variable-bar selection. Context is **read-only input to variable
resolution**, not a fourth store.

### 2. New variable kind: `context` (extends VARIABLES-AND-TEMPLATING)

`context` is a `VariableKind` with config `{ source: 'nav' | 'url' | 'tag' | 'values',
key }`:

- `nav` + `key=title|path[n]` → the node's title / an ancestor.
- `url` + `key=building` → bare `?building=…` (the `var-*` prefix stays variable state;
  `url` reads a **bare** param so external deep-links drive the board).
- `tag` + `key=building` → the board's `building` tag value.
- `values` + `key=site` → `PageContext.values[site]` (the node's `context.values`).

A `query` variable then references a `context` variable as a parent (`WHERE site =
$site`) — **no new interpolation**: it resolves to a value and rides the same
`variables` array and the same injection-safe engine already shipped
([VARIABLES-AND-TEMPLATING.md](./VARIABLES-AND-TEMPLATING.md) §2). Plus read-only
built-in tokens: `$__nav_title`, `$__tag(key)`.

### 3. Board tags (backend record-kind + UI)

- **Storage:** reuse the generic record model — a board's tags live in its content
  (`content.tags: { key: value }`) so they travel with the board, **or** as
  `rubix-core::tag` graph edges for cross-board reverse lookup. Tags are
  behaviour-affecting here (they feed `PageContext.tags`), so a write is a board edit
  and is gated/audited exactly as the board is (`IngestPublish`, the same authz the
  board's own record write uses). No new authz seam.
- **UI:** a `TagEditor` on the board; tags feed `PageContext.tags` and the `tag`
  source.

### 4. Navigation tree — a `kind:"nav_node"` record (backend contract + UI)

A nav node is a namespace-scoped, nestable record. Its `content`:

```jsonc
{
  "kind": "nav_node",
  "parent": "<nav_node id>" | null,   // self-ref; null = root; nestable arbitrarily deep
  "title": "Building 1",
  "sort_order": 10,
  "target": {                          // tagged union:
    "kind": "group"                    //   → header, non-clickable
    // | { "kind": "board", "board": "<board id>" }   → a reusable board mount
    // | { "kind": "route", "route": "devices" }      → a static app page
  },
  "context": { "values": { "site": "hq" }, "tags": {} },  // board targets only — { values?, tags? }
  "icon": null, "accent": null
}
```

- **`route` is a closed allow-list** of rubix's built-in pages (the router table) —
  e.g. `home | building | devices | dashboards | datasources | rules | sites | audit |
  access`. Not free-form. This lets a static page be mounted (and, when per-node ACLs
  land, gated) like a board.
- **Lifecycle on the records model:**
  - **CRUD + reorder/reparent** ride the generic `POST/PATCH/DELETE /records`
    (`?kind=nav_node` to list). The UI builds the tree from the flat node list via
    `parent`/`sort_order` (the same way it builds a board from `content.panels`).
  - **Board-target validation** is the one backend value-add over raw records:
    a `board` target's id should resolve to a `kind:"board"` record in the **caller's
    namespace**. The scoped session already confines reads to the namespace, so a
    cross-namespace board id simply does not resolve — validate in the UI on save
    (read the board by id on the scoped session; reject if absent). A future
    server-side hook on the record write can enforce it centrally if needed.
  - **Board delete → orphan nodes.** Deleting a board leaves its mounts dangling; the
    UI renders a dangling `board` target as a disabled/`group`-like node (losing a
    board must not hide the nav node). A server-side cascade is a follow-up (needs a
    delete hook on the generic record path).
- **Default tree seeded per tenant** (`crates/rubix-server/src/seed/`): every static
  `route` as a node so nothing is hidden out of the box; this is also where the demo
  fleet boards get their Building-1/Building-2 mounts. See "What to prove" #6.

### 5. Sidebar UI

`ui/src/features/nav/**` (new): a nested, collapsible tree replacing the flat
`shell/nav.ts` list. `group` nodes expand/collapse; `board`/`route` nodes are links.
An editor to add/nest/reorder nodes and pick a target (board + context, or a route).
Until per-node ACLs land, the tree shows every node in the namespace.

### 6. Routing & resolution wiring (UI)

- A `board` node opens `…/dashboards/:id?nav=:nodeId`; the board reads `nav`, loads
  the node, and merges `node.context` into `PageContext` (§1).
- Changing the node re-assembles context → updates the resolved `context`/`values`
  variables → bumps the same `varRevision` from
  [VARIABLES-AND-TEMPLATING.md](./VARIABLES-AND-TEMPLATING.md) §6 → re-queries exactly
  the dependent widgets. **Also re-key variable resolution on the assembled context**
  (nav/tag/url), or cascading option lists go stale when navigating between two nodes
  of the *same* board (the board id is unchanged — only the node differs).
- One revision, one dependency graph — do not add a parallel invalidation.

## What to prove

1. One board authored once, mounted at two nodes with different `context.values`,
   renders two sites' data — no second board.
2. `NavNode.context` merges per §1 (`values` → `PageContext.values`; `tags` over board
   tags).
3. Nav CRUD over `/records`: create / nest / reorder / reparent; group / board / route
   targets; deleting a board leaves its nodes intact (rendered dangling).
4. A node's `board` target id from another namespace does not resolve (scoped read).
5. Context in the query keys: changing the node bumps `varRevision` and re-keys
   resolution — two mounts never serve each other's cached widgets.
6. A fresh tenant gets a default tree of all routes; the demo fleet shows one board
   mounted under Building-1 (`site=hq`) and Building-2 (`site=…`).
7. Injection: a node title / context value with SQL metacharacters binds as a literal
   via the shipped engine, never executes.

## Acceptance criteria

- [ ] `PageContext` assembled from nav/url/tag/values with the §1 precedence.
- [ ] `context` is a `VariableKind` (config + resolver arm + form); resolves per
      source; reuses the shipped `variables` array + engine (no new interpolation).
- [ ] Board tags: stored on the board (content or tag edges), gated/audited as a board
      edit; `TagEditor` mounted; `$__tag(key)` and a `tag`-source variable resolve.
- [ ] `kind:"nav_node"` records via `/records`; group/board/route targets; UI builds
      the tree from `parent`/`sort_order`; board-target validated on the scoped
      session; dangling board target rendered, not lost.
- [ ] Nested sidebar tree replaces the flat list; node editor (add/nest/reorder/pick
      target + context).
- [ ] Context folded into `varRevision` + the variable-resolution key.
- [ ] Default tree seeded per tenant; demo fleet mounts one board at two nodes.
- [ ] Tests: PageContext precedence, context resolution per source, nav build from a
      flat record list, cross-namespace board-target non-resolution, context→key
      inclusion, injection.

## Departure / follow-up (explicitly deferred)

- **Per-node access control.** rubix has only namespace-level capability grants, no
  per-record ACL. Granting `view` per nav node (and gating static routes per user)
  needs a new subsystem: a `principal →can_view→ record` edge, a gate check, and a
  read-filter on the node list. Designed for here (the `route` allow-list and the
  default tree make every surface a node), implemented later. Until then, nav is
  namespace-wide and static pages stay reachable as today.
- **Server-side board-delete cascade / target FK** — needs a delete hook on the
  generic record path; the UI handles dangling targets in the interim.

## Out of scope (hand off)

- The variable model, interpolation engine, cascading →
  [VARIABLES-AND-TEMPLATING.md](./VARIABLES-AND-TEMPLATING.md) (this adds `context` as
  a source + the `$__` tokens only).
- Repeat-by-variable rendering → later (nav is navigation, not in-canvas repeat).
