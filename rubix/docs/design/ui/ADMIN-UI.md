# ADMIN-UI — reusable admin console on the rubix generic API

Design for a PocketBase-style admin console over rubix's generic REST/WS surface,
built so the **same components** drive both the admin console *and* any
domain app (EMS/BMS, project management, …) on the same backend. The **scope
authority** is [SCOPE.md](../SCOPE.md) ("the domain is not baked in; structure
comes from tagging on a graph") and the crate contracts are
[STACK-DEISGN.md](../../STACK-DEISGN.md). The agent runtime that shares this
substrate is [AGENT.md](AGENT.md). The operator-facing product that **embeds**
this admin console as one tenant-scoped section is [PRODUCT-UI.md](PRODUCT-UI.md).
Where this doc and SCOPE.md disagree, SCOPE.md wins.

## Thesis

The backend is already domain-agnostic — three schemaless tables (`record`,
`tag`, `tagged`), a generic CRUD surface (`/records`), a query surface
(`/query`), and a live plane (`/ws/records`). So the admin UI is **not** a
hand-built screen per entity; it is a **schema-driven renderer** over the
generic API. The same primitives that let an admin browse/edit any record let a
domain app render a "Sites" grid or a "Tasks" board — because a site, an
equipment, and a task are all just `record.content` JSON distinguished by a
`kind`/tag.

The reuse story is therefore **one component library, two consumers**: the admin
console is the first consumer and the reference implementation; a domain app is
the second. If the admin console needs a thing (a data grid bound to `/records`,
a JSON/schema-driven form, a tag picker, a live-updating table), the domain app
needs the same thing. We build it once.

**For now: one UI, two delivery targets — browser and desktop.** We are *not*
building two separate frontends. We build a single web UI from the packages
below and ship it both as a browser app (served by the backend) and as a
**Tauri** desktop app ([tauri-apps/tauri](https://github.com/tauri-apps/tauri)).
Same components, same `@rubix/*` packages, same React tree — the only difference
is how it's delivered and how it reaches the backend (see "Delivery targets").

## Stack (fixed)

Per direction: **shadcn/ui + Vite + TanStack + Tailwind CSS v4.** Concretely:

| Concern | Choice | Why it fits |
| --- | --- | --- |
| Build/dev | **Vite** | fast dev loop; static build embeds cleanly into the binary (below) |
| Components | **shadcn/ui** (Radix + Tailwind) | copy-in, not a dependency — components live in *our* tree, so they are forkable per consumer without fighting a black-box lib |
| Styling | **Tailwind CSS v4** | v4's CSS-first config (`@theme`) makes a shared design token layer a plain CSS import, no JS theme plumbing |
| Server state | **TanStack Query** | cache + invalidation for the REST surface; pairs with the WS live plane (live event → `queryClient.invalidateQueries`) |
| Tables | **TanStack Table** | headless grid — the load-bearing primitive for "render any record set as columns" |
| Routing | **TanStack Router** | type-safe routes; collection name is a route param (`/c/$kind`) so one route renders every collection |
| Forms | **TanStack Form** + a JSON-schema-driven field renderer | one form component renders any `kind`'s fields from its registered schema |

shadcn/ui is the keystone for the reuse goal: because its components are
**vendored into the repo** rather than imported, the shared package can own the
canonical copy and a consumer can override a single component without forking the
whole library.

## Monorepo shape — shared packages, one or many apps

The decision ("same UI vs. shared packages") resolves to **a pnpm/Turbo
workspace with shared packages and thin app shells** — this is the only shape
that makes the admin console and a domain app reuse code without copy-paste, and
it does *not* force a choice between "one app" and "many apps": apps are cheap
shells over the packages.

```
rubix/ui/
  packages/
    api/        @rubix/api      generated OpenAPI client + WS live hooks + TanStack Query hooks
    ui/         @rubix/ui       shadcn components + Tailwind v4 token layer (the design system)
    data/       @rubix/data     record-bound primitives: <RecordGrid>, <RecordForm>, <TagPicker>, <SchemaForm>, <LiveTable>
    schema/     @rubix/schema   collection/kind registry + JSON-schema types shared with the backend contract
  apps/
    web/        the UI as a browser app (served by rubix-server) — the one UI we build now
    desktop/    the SAME UI wrapped as a Tauri desktop app (thin shell, no UI fork)
```

There is **one UI app** of substance (`web`); `desktop` is a Tauri shell that
loads the identical build. Both pull all weight from the `@rubix/*` packages.
A `<domain>/` app (EMS/BMS or PM) is deferred — the packages support it, but for
now we push a single UI, not two.

- `@rubix/api` is **generated from the server's OpenAPI document** (utoipa already
  emits it — `rubix/crates/rubix-server/src/http`), so the client never drifts
  from the wire contract. It wraps each endpoint in a typed TanStack Query
  hook and exposes a `useLiveRecords()` hook over `/ws/records` that invalidates
  the matching query keys on each live event.
- `@rubix/data` is where the reuse pays off: `<RecordGrid kind="site">` and
  `<RecordGrid kind="task">` are the *same component*, differing only by the
  registered schema/columns for that kind.
- `apps/admin` is intentionally thin — routes + layout + a generic
  collection screen. Almost all of its weight lives in `@rubix/data`.

**Single deployable, still.** For browser delivery the `web` app's static build
is **embedded into `rubix-server`** (e.g. `rust-embed`) and served at `/admin`,
preserving the "same binary on edge or cloud" contract from STACK-DEISGN. Dev
runs Vite standalone against the API; the build step bundles assets the server
includes. Desktop delivery (Tauri) bundles the **same** static build in its
webview — see "Delivery targets". The packages don't care which target consumes
them.

## Delivery targets — browser and desktop, one UI

The same web build is delivered two ways. The UI code does not branch on target
beyond a single injected "where is the backend" config.

| Target | What ships | How it reaches the backend |
| --- | --- | --- |
| **Browser** | static build embedded in `rubix-server`, served at `/admin` | same-origin REST/WS to the backend that served it — **the backend is the server**; no separate API host |
| **Desktop** | Tauri app bundling the same static build in its webview | REST/WS over the network to a configured rubix endpoint (local edge box or cloud) |

**Browser mode is a true client/server split.** The browser holds no data and no
logic beyond rendering — it is a thin client of `rubix-server`'s generic API.
"Proper backend + UI" is exactly the current shape: the backend owns the store,
the gate, auth, and the query/live planes; the browser UI is one more API
consumer (alongside the agent runtime in [AGENT.md](AGENT.md)). This is the
PocketBase model — one binary serves both the API and the admin UI.

**Desktop mode (Tauri) is the same UI in a native shell.** Tauri loads the
identical web build in its system webview; the only real difference is that it
talks to the backend **over the network** rather than same-origin, so it needs a
configured endpoint and the backend must permit its origin (the dev-proxy/CORS
open question, now also a packaged-app concern). Two desktop shapes are possible
and the doc does not pick yet (open question 8):

- **Thin desktop** — the app is purely a client; it points at a remote/edge
  `rubix-server`. Smallest shell, nothing Rust-side beyond Tauri.
- **Self-contained desktop** — Tauri's Rust side **embeds/launches a local
  `rubix-server`** (the edge profile, embedded SurrealDB) so the desktop app is a
  full offline-capable node. This reuses the exact same binary/profile the edge
  deployment already builds (STACK-DEISGN: edge = single namespace, embedded
  engine) — Tauri just supervises it. Heavier, but it makes "rubix on a laptop"
  a single installable.

Because Tauri's host process is Rust, the self-contained shape is a natural fit —
it can depend on the rubix crates directly rather than shelling out. That choice
is deferred; the UI packages are identical either way.

## The schema layer — what makes generic screens possible

The backend is schemaless by design, but a renderer needs *some* shape to draw a
grid column or a form field. The resolution is a **collection registry** that
lives in `@rubix/schema` and (eventually) is **served by the backend** so it is
data, not hardcoded UI:

- A **collection** = a `kind` value + a JSON Schema for that kind's `content` +
  display hints (columns, labels, which fields are list-visible). This is the
  PocketBase "collections" concept mapped onto rubix's tag graph: a collection is
  a saved convention over generic records, **not** a new table.
- The admin "Collections" screen is itself CRUD over collection-definition
  records (a `kind: "collection"` record) — so defining a new collection is the
  same generic write path as any other record. No backend code change to add a
  domain.
- Until the backend serves collection definitions, the registry is a local
  config object with the same shape — the screens don't change when it moves
  server-side, only the source of the registry does.

This is the one piece that turns "generic record store" into "build the
backend/schema from the UI": the user defines collections (kinds + field
schemas) in the admin console, and every generic screen immediately renders them.

## Reusable component taxonomy (`@rubix/data`)

Each is bound to the generic API, parameterized by a collection from the schema
registry — the same component serves admin and domain apps.

| Component | Backs onto | Reused for |
| --- | --- | --- |
| `<RecordGrid kind>` | `GET /records` (filtered by kind/tag) + TanStack Table | every list screen — admin record browser, "Sites", "Tasks" |
| `<LiveTable>` | `<RecordGrid>` + `useLiveRecords()` | any screen that must reflect live changes (readings, run status) |
| `<SchemaForm collection>` | `POST`/`PATCH /records` + JSON Schema | create/edit any record from its kind's field schema |
| `<RecordDetail id>` | `GET /records/:id` | inspect/edit one record; JSON view fallback for unknown shapes |
| `<TagPicker>` | tag set on the record (the `tagged` graph) | classify records; the structure-by-tagging primitive |
| `<QueryConsole>` | `POST /query` (DataFusion) | admin SQL console; domain analytics widgets |
| `<DatasourceList>` | `GET /datasources` | admin datasource view; "connected sources" in a domain app |
| `<CollectionEditor>` | collection-definition records | define a kind + its field schema (the schema-from-UI surface) |
| `<JsonContentField>` | raw `content` JSON | escape hatch for any field the schema doesn't model — never blocks an unknown shape |

The escape hatch matters: because the store is genuinely schemaless, every
schema-driven screen falls back to raw JSON for fields the registered schema
doesn't cover, so a half-defined collection is still fully editable.

## Admin console screens (first consumer)

- **Records browser** — `<RecordGrid>` across all kinds, namespace-scoped to the
  principal; filter by tag/kind; drill into `<RecordDetail>`.
- **Collections** — list/define collections via `<CollectionEditor>`; this is the
  "build the schema from the UI" surface.
- **Query console** — `<QueryConsole>` over `/query`.
- **Datasources** — `<DatasourceList>`; register-datasource flow gated by the
  `datasource-register` capability.
- **Auth/principals** — surface the principal + capability grants (read-only
  first; grant management is a gated mutation, later).
- **Live monitor** — `<LiveTable>` demonstrating the WS plane.

## Contracts honored

- **Generic-by-construction (SCOPE principle 4)** — no screen hardcodes a domain
  type; every entity is a `record` distinguished by `kind`/tags. The same
  components render EMS points and PM tasks.
- **Two enforcement points (STACK-DEISGN contract #1)** — the UI is a pure client
  of the existing seams: every mutation goes through `POST/PATCH/DELETE /records`
  (the gate), every read through the scoped-session endpoints. The UI adds **no**
  new authz path and must not assume a grant the API doesn't return.
- **Capability surface unchanged** — gated actions (datasource register, external
  query) map to the existing five capability grants; the UI reflects grants, it
  does not invent them.
- **Same binary (STACK-DEISGN)** — embedded static build keeps edge/cloud a single
  artifact.
- **Contract drift impossible** — the client is generated from the server's
  OpenAPI document, not hand-written.

## Open questions

1. **Collection definitions: client config vs. server-served.** Start as a
   `@rubix/schema` config object, or land a `kind: "collection"` record + a
   read endpoint first? Server-served is the end state (schema-from-UI); decide
   whether the first cut ships client-side and migrates, or waits for the
   endpoint.
2. **List filtering by kind/tag.** `GET /records` currently lists all readable
   records; a domain grid needs server-side filter by kind/tag (or it filters
   client-side at small scale first). Decide the query param shape (and whether
   it routes through `/query` instead for non-trivial filters).
3. **Field-level schema in the backend.** Is per-kind validation enforced
   server-side (a "kind contract" layer, the gap noted in the reuse review) or
   only advisory in the UI at first? If server-side, it is a backend change with
   its own design, not a UI concern.
4. **Auth surface for the UI.** How does the console authenticate (session token
   issuance) and how are a principal's grants exposed for the UI to reflect?
   Needs a server endpoint that returns the current principal + capabilities.
5. **Relations in the UI.** Record→record links are JSON fields today, not graph
   edges (the relations gap from the reuse review). Decide whether the UI models
   relations as tag/edge picks or stays with id-in-content until the backend adds
   first-class relations.
6. **One app or many.** The packages support both; decide whether a domain app
   ships in this repo as a second `apps/*` consumer (proves reuse) or lives
   downstream (packages published).
7. **Embed mechanism + dev proxy + desktop origin.** `rust-embed` vs. alternative
   for the browser static bundle, the dev-time CORS/proxy story between Vite and
   the API, and the CORS/origin allowance the packaged Tauri app needs when it
   talks to the backend over the network.
8. **Desktop shape (Tauri).** Thin client pointed at a remote/edge backend, vs.
   self-contained desktop that embeds/launches a local edge `rubix-server`
   (reusing the edge profile + embedded SurrealDB) for an offline-capable node.
   The UI packages are identical either way; the decision is Rust-side host
   scope, not UI work.
