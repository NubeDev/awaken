# PRODUCT-UI — the operator console (copilot-first, tenant-scoped)

The end-user-facing UI for rubix: a **copilot-first operator console** where the
agent is the front door and every screen hangs off a logged-in **tenant**. This
is the experience captured by the `ui-demo/rubix-v2/rubix-copilot` prototype —
the look, density, and interaction model we are building toward.

It is the **companion** to [ADMIN-UI.md](ADMIN-UI.md), not a replacement.
ADMIN-UI describes the *generic admin console* — a schema-driven renderer over the
raw `/records` / `/query` / `/ws/records` surface (the PocketBase-style substrate).
This doc describes the *product the operator actually opens*: a portfolio of sites,
an attention queue, dashboards, and a conversational agent. The two share one
component library and one backend; the admin console is **embedded inside** this
product as one capability-gated section, not a separate app.

Authority order, on conflict: [SCOPE.md](../../SCOPE.md) (tenancy + generic-by-
construction) wins, then [STACK-DEISGN.md](../../STACK-DEISGN.md) (crate contracts),
then ADMIN-UI.md (component library), then this doc.

## Thesis

Two ideas drive the whole UI:

1. **The agent is the front door.** The operator does not hunt through screens for
   problems — rubix surfaces them, ranked by impact, in an *attention queue*, and
   the operator acts (or asks) from there. "Two things need you" is the landing
   state, not a buried alert list. Acting resolves the item and shows the
   consequence inline (fail over → recovery chart → queue clears). The demo's
   copilot screen is the reference: a thread on the left, the impact-ranked queue
   on the right, an ask bar that both answers questions *and* executes actions.

2. **Everything is tenant-scoped, and the URL says so.** A user logs into a
   **tenant** (a namespace, per SCOPE.md) and every read, write, live feed, route,
   and breadcrumb is bound to it. Tenancy is not a setting buried in a header — it
   is the first path segment, so a URL is shareable, bookmarkable, and unambiguous
   about *which tenant's data you are looking at*.

Both ideas sit on the **same generic backend** the admin console uses. A site, a
zone, a rule, a device is a `record` distinguished by `kind`/tags
([BACKEND-COLLECTIONS.md](../BACKEND-COLLECTIONS.md)); the copilot's answers and
the dashboards' widgets are renderings of `/records` + `/query` + the live plane.
No domain type is baked into the backend — the EMS/BMS shape in the demo is a
*collection convention*, exactly as ADMIN-UI describes.

## Multi-tenancy — the load-bearing constraint

rubix is multi-tenant by construction (SCOPE.md: "namespace (tenant)"; cloud is
namespace-per-tenant, edge is the degenerate single-namespace case). The UI must
treat the tenant as ambient, scoped context — never as data mixed across tenants.

- **Tenant = namespace.** The thing the user "logs into" and the thing every
  screen is scoped to is a SurrealDB namespace. The backend already signs scoped
  read sessions into a namespace (`crates/rubix-server/src/auth.rs`,
  `AppState::namespace`); today that namespace is server-instance config, and the
  target below makes it request-scoped per tenant.
- **A principal may see one or many tenants.** A single-site operator has one
  tenant; a portfolio manager or MSP has several. The **Sites/portfolio screen is
  really a tenant (or site-within-tenant) picker** — the demo's portfolio grid is
  the entry point, and opening a card enters that tenant's scope.
- **Isolation is the backend's job, reflected by the UI.** The UI never filters one
  tenant's data out of another's response — it only ever asks the backend for *the
  current tenant*, and the backend (gate + namespace scoping) guarantees a
  principal cannot read across the boundary. The UI adds **no** cross-tenant path
  (mirrors ADMIN-UI's "two enforcement points" contract).
- **Edge stays single-tenant for free.** On an edge node the gate resolves to the
  one namespace (SCOPE.md); the tenant segment is still present in the URL but
  fixed, so the *same* routing code runs on edge and cloud with no branch.

## Routing — tenant-scoped, mirroring the backend

The backend API is versioned and tenant-scoped:

```
/api/v1/t/<tenant>/records
/api/v1/t/<tenant>/records/:id
/api/v1/t/<tenant>/query
/api/v1/t/<tenant>/datasources
/api/v1/t/<tenant>/ws/records          (live plane, same scoping)
```

The frontend route tree **mirrors it one-to-one** so the URL bar and the API calls
share a single source of truth for "which tenant":

```
/                                      → tenant/site picker (the portfolio screen)
/t/<tenant>                            → Home hub for that tenant
/t/<tenant>/copilot                    → Ask Rubix (thread + impact queue)
/t/<tenant>/insights                   → Insight Center
/t/<tenant>/overview                   → pinned board
/t/<tenant>/dashboards/:id             → saved dashboard
/t/<tenant>/building                   → building & zones (domain collection view)
/t/<tenant>/rules                      → rules
/t/<tenant>/data                       → data sources
/t/<tenant>/reports                    → reports
/t/<tenant>/devices                    → devices
/t/<tenant>/settings                   → site & team
/t/<tenant>/admin/...                  → the embedded admin console (see below)
```

- `<tenant>` is a **route param**, resolved once into an API base
  (`/api/v1/t/<tenant>`) and a WS endpoint, then threaded through every `@rubix/api`
  hook and the `useLiveRecords()` subscription. Changing tenant is changing the
  param — no global mutable "current tenant" outside the router.
- With **TanStack Router** (the stack ADMIN-UI fixes), this is a `/t/$tenant`
  layout route whose loader resolves the principal's access to that tenant and
  exposes the scoped API client to all children via context. A principal hitting a
  tenant they cannot access fails closed (404/redirect to the picker), matching the
  gate's fail-closed posture.
- **Same-origin in browser, configured-endpoint on desktop** (ADMIN-UI delivery
  targets): only the host of `/api/v1/...` changes; the `/t/<tenant>/...` suffix is
  identical on both targets.

### Backend gap (call it out, don't hide it)

Today the server mounts **flat, unversioned, single-namespace** routes — `/records`,
`/query`, `/datasources`, `/ws/records` — with the namespace fixed in `AppState`
and tenancy carried implicitly by the credential's scoped session
(`crates/rubix-server/src/http/mod.rs`, `auth.rs`). The `/api/v1/t/<tenant>` shape
is a **target**: it requires the backend to (a) version under `/api/v1`, (b) nest
routes under a `t/:tenant` param, and (c) resolve the tenant param to a namespace
and validate the authenticated principal's access to it (rejecting mismatches at
the gate, not in the UI). That is a backend change with its own design — the UI is
built against this contract, and until the backend lands it, an `@rubix/api`
adapter maps the tenant-scoped client onto the current flat routes so the UI does
not encode the old shape.

## How the demo maps to the build

The prototype is a throwaway artifact (Tailwind CDN + vanilla JS + hardcoded
`RX.*` data) — **none of its code ports**. What ports is the *design*: the screen
inventory, the interaction model, the visual language, and the information density.
Each demo file maps to a real component or package:

| Demo (`copilot/*.js`) | Real home | Notes |
| --- | --- | --- |
| `data.js`, `model.js` (hardcoded sites/zones/insights) | `@rubix/api` hooks over `/records` + `/query` | becomes live, tenant-scoped data; the hardcoded shapes become `kind` collections |
| `screens.js` (sites, home, dashboards, building, pages) | `apps/web` routes + `@rubix/data` primitives | sites→tenant picker; dashboards→`<RecordGrid>`/widget board; building→a domain collection screen |
| `app.js` (router, copilot thread, queue, pins) | `apps/web` copilot route + a thread/queue feature module | the attention queue is a ranked view over agent-surfaced `record`s; pins persist server-side, scoped to tenant |
| `answers.js` (intent routing, answer composer) | the **agent runtime** ([AGENT.md](../AGENT.md)) | the demo fakes intent routing client-side; real answers come from the agent over the same API |
| `viz.js` (SVG line/spark/gauge/donut/bars/heat) | `@rubix/ui` chart primitives | the one part nearly portable as-is conceptually; reimplemented as React/Tailwind v4 components |

The demo's strongest, must-keep moments:

- **The attention queue** ("Rubix lined up · by impact") as the default landing
  state — ranked, act-from-here, resolves inline.
- **Act → consequence → resolve**: a primary action runs, shows a before/after viz,
  and clears the queue item (precool/failover in `app.js`).
- **"Build me a live view" → pin it**: the agent composes a live dashboard tile set
  and the operator pins it to a persistent, tenant-scoped board.
- **Omni-search (⌘K)** that jumps across tenants, screens, dashboards, and zones —
  and falls through to "ask Rubix" when nothing matches.
- **Visual language**: orb identity, serif for the agent's prose, mono tabular
  numerals, heat-mapped zone cells, hand-tuned chart density.

## Stack and packages (inherited from ADMIN-UI)

This product is built from the **same** monorepo and packages ADMIN-UI fixes —
shadcn/ui + Vite + TanStack + Tailwind v4, with `@rubix/api` generated from the
server's OpenAPI document. It does not introduce a parallel stack. It adds:

- a **copilot/agent** feature surface (thread, impact queue, act-and-resolve,
  pinning) that consumes the agent runtime ([AGENT.md](../AGENT.md)) and the same
  `/records` live plane, and
- a **tenant layout route** (`/t/$tenant`) that all product *and* admin screens
  nest under.

`apps/web` is the single UI app (browser + Tauri desktop, per ADMIN-UI delivery
targets). The admin console is not a second app — it is the `/t/<tenant>/admin/...`
section of this one, gated by the admin capability.

## Admin console — embedded, not separate

Per the decision: the admin section is **added into this UI**. It is the
`/t/<tenant>/admin/*` route subtree and uses the exact `@rubix/data` primitives
ADMIN-UI defines (`<RecordGrid>`, `<SchemaForm>`, `<CollectionEditor>`,
`<QueryConsole>`, `<DatasourceList>`, `<TagPicker>`). Concretely:

- The admin screens from ADMIN-UI (Records browser, Collections, Query console,
  Datasources, Auth/principals, Live monitor) live under `/t/<tenant>/admin` and
  are scoped to the **same tenant** the operator is in — admin is "manage *this*
  tenant's substrate", not a god-mode across tenants.
- Cross-tenant administration (provisioning a new tenant/namespace, MSP-level
  views) is a **separate concern** and out of scope here — it would be its own
  capability and likely its own top-level area, not folded into a tenant's admin.
- Reaching admin requires the admin capability; the UI reflects the grant, it does
  not invent it (ADMIN-UI "capability surface unchanged").

This is the reuse payoff: the operator's "Building & Zones" screen and the admin's
"Records browser" are the *same* `<RecordGrid>` bound to different collections in
the same tenant.

## Contracts honored

- **Tenant isolation (SCOPE.md).** Every route, read, write, and live subscription
  is scoped to one namespace; the UI never crosses the boundary and relies on the
  gate to enforce it. URL `/t/<tenant>` ≙ API `/api/v1/t/<tenant>` ≙ namespace.
- **Generic-by-construction (SCOPE principle 4).** No screen hardcodes a domain
  type into the *backend*; EMS/BMS entities are `kind` collections. The same
  components render this product and the admin console.
- **Two enforcement points (STACK-DEISGN contract #1).** The UI is a pure client of
  the existing seams — mutations through the gate, reads on the scoped session. It
  adds no authz path and assumes no grant the API does not return.
- **Same binary (STACK-DEISGN).** Browser build embeds into `rubix-server`; desktop
  reuses the same build. Tenancy in the path does not change the artifact.
- **Contract drift impossible for the generated surface.** `@rubix/api` is
  generated from the server OpenAPI document, so endpoint shapes cannot drift. The
  one hand-maintained seam is the tenant wrapper: today's OpenAPI describes flat
  routes, so the `/t/<tenant>` shape and the tenant→namespace resolution live in a
  hand-written adapter (above), not the generated contract — for the tenancy
  dimension, drift is exactly what is possible until the backend versions the
  routes and regenerates the client.

## Open questions

1. **Tenant resolution endpoint.** The `/t/$tenant` layout loader needs "which
   tenants can this principal access, and does it have admin on each?" — a server
   endpoint returning the principal + its accessible namespaces + per-namespace
   grants. (Extends ADMIN-UI open question 4.)
2. **Backend route reshape.** Sequencing the move to `/api/v1/t/:tenant/...`
   (versioning + tenant param + namespace resolution at the gate) vs. the
   `@rubix/api` adapter that maps onto today's flat routes in the interim.
3. **Tenant switching UX.** In-app switch (re-resolve the param, refetch) vs.
   full reload; how the omni-search jumps *across* tenants given each is a separate
   namespace/scope.
4. **Pin & board persistence per tenant.** The demo persists pins in
   `localStorage`; real boards are tenant-scoped `record`s (a `kind:"board"`
   collection) so they follow the user across devices and respect isolation.
5. **Agent surface boundary.** Which copilot answers/actions are agent-runtime
   ([AGENT.md](../AGENT.md)) vs. plain `/query` calls — i.e. where the line sits
   between "ask Rubix" (LLM/agent) and a deterministic dashboard query.
6. **Cross-tenant / MSP area.** Whether portfolio-wide or provider-level
   administration ever becomes a first-class area above the tenant scope, or stays
   out of this product entirely.
