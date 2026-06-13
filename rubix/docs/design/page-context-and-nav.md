# Page Context & Navigation — reuse one dashboard across a fleet

Scope for making **one dashboard serve a whole fleet** by separating the *page* (a
parameterised template) from the *place* it's viewed for (a site, a building, a
level). Today a board is reused only by hand-picking values; rubix can't even do that
yet (no variables — see [variables-and-templating.md](variables-and-templating.md)).
This doc adds the two coupled pieces that finish the fleet story at the navigation
layer, mirroring nexus WS-13 re-grounded in rubix's **org/site + RBAC** model.

1. **Page context** — a page reads the **URL**, its **nav-tree position**, and its
   **tags**, and feeds those into variables/queries. So one `energy-overview` board
   rendered under *Building-1* queries Building-1's data; the same board under
   *Building-2* queries Building-2's — no second board authored.
2. **A navigation tree** — a nested, user-built nav where each node **assigns a board
   and a context payload**. `Buildings → Building-1 (energy-overview, {site: s1}) →
   Level-1`. Click a node → open its board bound to its context.
3. **Nav-based access** — because the node (not the board) is what a user navigates,
   **access is granted per node**, for both dashboard pages and static app pages.

Together: author one board, mount it at many nodes, each node parameterises *and*
gates it.

## Problem / current state

- **Navigation is a flat, scope-derived sidebar.** `ui/src/components/layout/app-sidebar.tsx`
  builds nav groups from `scopedNavGroups(org, siteSlug, …)`; there is no nesting, no
  user-built nodes, no per-node context. Dashboards are listed flat under the org/site
  scope.
- **"Context" today is only URL-path scope.** `ui/src/context/scope-provider.tsx`
  resolves `org` + optional `siteSlug` from the route into a `Site`. There is **no
  source that reads bare URL params, a nav node, or tags** — that's the gap.
- **No variables to feed.** This doc's context *resolves into* the variable layer,
  which [variables-and-templating.md](variables-and-templating.md) introduces. Build
  that first; context is a new *source* for it.
- **No tags on dashboards.** Rubix has tag-driven point selection
  (`ui/src/api/tags.ts`) but no general entity-tag store and no dashboard tags. Tags
  here are a new, behaviour-affecting input — they need their own small store + authz.
- **Access is RBAC grants on resource refs**, not per-page sharing
  (`docs/design/authz-rbac.md`, `crates/rubix-server/src/api/grants/`). The grant
  model addresses resources by `resource_ref`; this doc adds a `nav_node` resource
  kind and makes nav nodes the navigation-grant surface.

> **rubix vs nexus.** Nexus assumes Postgres RLS and a generic authz seam with
> `InstancesProvider`s. Rubix is SQLite-edge / Postgres-cloud with **RBAC grants and
> org-scoping** (no RLS). So: the nav table is org-scoped (not RLS-policied); access
> enforcement reuses the existing grant check in `api/grants/` + the two-layer authz
> in `docs/design/authz-rbac.md`; there is no `InstancesProvider` to mirror — register
> `nav_node` as a grantable resource kind in the existing grant model instead.

## Scope

### 1. Page context model

A page resolves against a `PageContext` assembled at view time from sources kept
**separate** (not pre-flattened) so a `context` variable can address exactly one and
precedence is explicit and testable:

```
PageContext {
  nav?:   { node_id, slug, name, path: string[] },  // the node the page opened under
  url:    Record<string, string | string[]>,        // query params (bare + var-*)
  tags:   Record<string, string | null>,            // the board's tags
  scope:  { org, site_id? },                         // rubix's existing URL scope
  values: Record<string, string | string[]>,        // NavNode.context.values overrides
}
```

**`NavNode.context`** is exactly `{ values?, tags? }` — not arbitrary top-level keys:

- `context.values` → `PageContext.values` (a node `{ values: { site: "s1" } }` makes
  `$site` resolvable via a `context`/`values` variable). This is the override path.
- `context.tags` → merged **over** the board's own tags into `PageContext.tags` (a
  node can pin a tag for its mount without retagging the shared board).

**Precedence (later wins), per variable name:** board tags → `nav.values` → URL params
→ explicit variable-bar selection. Context is **read-only input to variable
resolution**, not a fourth store. Assembled in `ui/src/features/variables/context.ts`
(new) and threaded into the resolution from
[variables-and-templating.md](variables-and-templating.md).

### 2. New variable kind: `context` (extends the variables doc)

`context` is a first-class `VariableKind`. Config:
`{ source: 'nav' | 'url' | 'tag' | 'values', key }`:

- `nav` + `key=slug|name|path[n]` → the node's slug/name/ancestor.
- `url` + `key=building` → bare `?building=…` (the `var-*` prefix stays variable-state;
  `context`/`url` reads a **bare** param so external deep-links drive the page without
  knowing variable internals).
- `tag` + `key=building` → the board's `building` tag value.
- `values` + `key=site` → `PageContext.values[site]` (the node's `context.values`).

A `query` variable then references a `context` variable as a parent
(`WHERE site_id = '$site'`) — no new interpolation, it rides the same injection-safe
binder. Plus built-in tokens (no authoring): `$__nav_slug`, `$__nav_name`,
`$__tag(key)`.

Adding the kind is the full-stack checklist from the variables doc: Rust DTO enum +
config struct (`crates/rubix-core/src/model.rs`), `api/openapi.rs` registration, TS
union + parser + `resolve.ts` arm + editor form, export/import, tests at each layer.

### 3. Dashboard tags + close the tag authz gap

- **Backend (new):** a small org-scoped entity-tag store `(org, kind, entity_id, key,
  value)` and routes `PUT /api/v1/tags/{kind}/{id}` (full-replace),
  `GET /api/v1/tags/{kind}/{id}`, `GET /api/v1/tags/entities/{kind}` (reverse lookup),
  `GET /api/v1/tags/keys`. Because tags are now **behaviour-affecting** (they drive
  queries via `PageContext.tags`), the write/read handlers must resolve the target
  entity and enforce the **same authz the entity's own routes use** (`edit` to write,
  `view`/read scope to read, via `docs/design/authz-rbac.md` + `api/scope_auth.rs`),
  and reject unknown/foreign ids. Sweep tags on entity delete (in the existing delete
  handlers, the same place cascades happen).
- **Frontend:** a `TagEditor` mounted on the board with `kind="dashboard"`; tags feed
  `PageContext.tags` and the `tag` context source.

### 4. Navigation tree — the single navigation + access surface

New backend resource + sidebar UI. A nav node is org-scoped and nestable:

```
NavNode {
  id, org,
  parent_id?,          // self-ref, NULL = root; nestable arbitrarily deep
  title, sort_order,
  target: jsonb,       // tagged union:
                       //   { kind: "group" }                          → header, non-clickable
                       //   { kind: "dashboard", dashboard_id }         → a reusable board mount
                       //   { kind: "route", route: "datasources" }     → a static app page
  context?: jsonb,     // dashboard targets only — exactly { values?, tags? } (§1)
  icon?, accent?,
}
```

- The `route` kind is a **closed allow-list** of rubix's built-in pages (the router
  table) — e.g. `sites | equips | points | dashboards | datasources | rules | boards |
  sparks | runs | audit | access`. Not free-form. This lets a static page (Datasources,
  Audit) be access-gated by a node exactly like a board.
- **Migration:** a `nav_nodes` table via a forward-only `user_version` bump
  (`crates/rubix-server/src/store/migrate.rs`), dual-dialect (SQLite + Postgres cloud),
  org-scoped, indexed on `(org, parent_id, sort_order)`. `target` is JSONB; a
  `dashboard` target's id is validated **in the handler** to exist within the caller's
  **org** (a bare FK can't encode org-safety since `dashboards.id` is a global PK with
  `org` a separate column — `model.rs:121-146`). On board delete, sweep dependent nodes
  to `{ kind: "group" }` (in `store/dashboards.rs` delete path) — losing a board must
  not delete the nav node.
- **CRUD + reorder/reparent** routes under `api/nav/**`, DTO registered in
  `api/openapi.rs`, TS mirror hand-authored in `ui/src/api/types.ts`.
- **Sidebar UI** (`ui/src/features/nav/**`): a nested, collapsible tree replacing the
  flat `app-sidebar.tsx` dashboards list; `group` nodes expand/collapse,
  `dashboard`/`route` nodes are links. The signed-in user sees **only nodes they hold
  `view` on** (server-filtered by the grant check). An editor to add/nest/reorder
  nodes, pick a target (board + context, or a static route), and manage grants inline.
- **Routing:** a `dashboard` node opens `…/dashboards/:slug?nav=:nodeId` → the page
  reads `nav` → loads the node → merges `node.context` into `PageContext` (§1). A
  `route` node navigates to that static page. Same components; only the entry point and
  (for boards) the context differ.

### 5. Resolution wiring — context must be in the query keys

On load: assemble `PageContext` → seed `context` variables + bare URL params → resolve
the rest (cascading) in the variables-doc order. The re-query path is revision-based
(`varRevision` from the variables doc), so context must feed it:

- Fold resolved context **into the variable values** that drive `varRevision`, so a
  context change bumps it like any selection. Route everything through variables so
  there is **one** revision to bump.
- Re-key variable *resolution* on the assembled context too (nav/tag/url), or option
  lists for cascading `query` variables resolve stale when navigating between two nodes
  of the same board (the board slug is unchanged — only the node differs).

Changing the node re-assembles context → updates selections → bumps `varRevision` →
re-queries exactly the dependent widgets. Reuse the variables doc's
dependency-driven invalidation; do not add a parallel one.

### 6. Access model — grant on the nav node

Move the navigation access surface to the node, reusing rubix's existing grant model
(`api/grants/`, `docs/design/authz-rbac.md`) rather than inventing a parallel one:

- Register `nav_node` as a grantable resource kind (`view | edit | delete`,
  org-scoped) in the grant/resource-ref model.
- The nav-tree `GET` filters to nodes the principal holds `view` on (the existing
  grant check, applied per node — the read-filter pattern already used elsewhere,
  `docs/design/crud-and-tenancy.md`).
- Opening `…/dashboards/:slug?nav=:id` checks `view` on the **node**, not the board.
  A `route` node checks `view` on the node. Boards/datasources keep their own
  `edit`/`delete` grants for *authoring* (who may modify the template) — `view`-to-
  navigate becomes a node grant.
- **Static pages get gated for the first time.** Seed a **default tree** on org
  provision (`api/orgs/create.rs`) — every static route as a node, granted org-wide —
  so nothing silently disappears; gating is opt-in tightening, not a lockout.

### 7. UX workflow

**A. Admin builds navigation** (new Navigation builder, `ui/src/features/nav/**`):
add a group ("Buildings"); add a board mount ("Building-1" → board `energy-overview` →
context `values: { site: "s1" }`); repeat "Building-2" → same board → `site: "s2"`
(one board, two mounts); add a static page ("Datasources" → route `datasources`); nest
/ reorder by drag; grant access per node inline.

**B. End user navigates:** sidebar shows only granted nodes; clicking "Building-1"
opens `energy-overview` bound to `site=s1`; "Building-2" → same board, `site=s2`,
re-queried; a deep link `…/dashboards/energy-overview?nav=<building-1>` restores the
exact view (access-checked on that node).

**C. Access administration** (the Access section): a Navigation tab lists the tree;
each node shows scope + grant count + manage. Authoring grants live under each asset's
own admin surface.

## Design notes

- **Nav tree ≠ scope.** Rubix's org/site scope *files* an entity once; a nav node
  *mounts* a (possibly shared) board with a context. The same board legitimately
  appears under Building-1 and Building-2 nodes — that's the point. Build a separate
  `nav_nodes` table; do not overload the scope/site model.
- **Context is precedence-merged, never a 4th store.** Tags persist in the tag store,
  nav context in `nav_nodes.context`, URL in the URL — `PageContext` is the *resolved
  view* at render time, keeping board export/import self-contained.
- **Injection boundary unchanged.** Context values become variables and bind as
  parameters via the variables engine — a node titled `'); DROP …` is quoted, never
  executed.
- **Bare URL params vs `var-*`.** Variables own `?var-site=…`; this doc's `url` source
  reads **bare** `?building=b1` for external deep-links. Don't collide the prefixes.

## What to prove

1. One board authored once, mounted at two nodes with different `context.values`,
   renders two different sites' data — no second board.
2. `NavNode.context` merges per §1 (`values` → `PageContext.values`, `tags` over board
   tags).
3. Nav CRUD: create/nest/reorder/reparent; group/dashboard/route targets; deleting a
   board sweeps its nodes to `group` (no nav rows lost).
4. A nav node cannot reference another org's board (handler validates org-scoped
   existence).
5. Nav-based access: the sidebar `GET` is filtered to nodes the principal holds `view`
   on; opening a node checks `view` on the node. A user granted Building-1 but not
   Building-2 sees and opens only Building-1 though both reuse one board.
6. Static pages gated: a `route` node hidden from an ungranted user; a fresh org gets
   a default tree of all routes (granted org-wide).
7. Context in the query keys: changing the node bumps `varRevision` and re-keys
   variable resolution — two mounts never serve each other's cached widgets.
8. Tag authz: a caller can't tag a board they can't edit or a nonexistent id.
9. Deep-links (`?nav=…` and bare `?building=…`) restore the view.
10. Injection: a nav title / tag value with SQL metacharacters binds, never executes.

## Acceptance criteria

- [ ] `PageContext` assembled from nav/url/tag/scope/values with the §1 precedence;
      no `varOverrides` channel.
- [ ] `context` is a full-stack `VariableKind` (DTO + OpenAPI + TS + resolve + form +
      export/import); resolves per source; a `query` panel re-queries on node change;
      cascading from a `context` parent works.
- [ ] Dashboard tags: org-scoped tag store + routes; write/read enforce the entity's
      own authz and reject unknown/foreign ids; swept on delete; `TagEditor` mounted;
      `$__tag(key)` and a `tag`-source variable resolve.
- [ ] `nav_nodes` table via forward-only migration, dual-dialect, org-scoped; group/
      dashboard/route targets; org-scoped dashboard-target validation; board delete
      sweeps nodes to `group`.
- [ ] Nav CRUD + reorder/reparent routes; nested sidebar tree replaces the flat list.
- [ ] `nav_node` registered as a grantable kind; sidebar `GET` filtered to `view`;
      opening a node checks node `view`; default tree seeded on org provision.
- [ ] Context folded into `varRevision` + variable-resolution key.
- [ ] Audit/undo: nav create/update/delete/reorder + grants recorded + undoable; tag
      edits recorded under the dashboard kind (confirm with
      [audit-and-undo.md](audit-and-undo.md), no double-record).
- [ ] Tests: PageContext precedence, context resolution per source, nav CRUD + reorder
      + org isolation + cross-org rejection, tag authz, context→key inclusion,
      dependency-driven re-query on nav change, injection.

## Out of scope (hand off)

- The variable model, interpolation, cascading engine →
  [variables-and-templating.md](variables-and-templating.md) (this adds `context` as a
  source + the `$__` tokens only).
- Time range → [time-range-and-refresh.md](time-range-and-refresh.md).
- Audit/undo substrate → [audit-and-undo.md](audit-and-undo.md) (this wires its own
  `record` calls + reverser per C6).
- Repeat-by-variable rendering → later (nav is navigation, not in-canvas repeat).
