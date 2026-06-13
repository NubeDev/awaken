# Feature — Entity CRUD & Org/Site Tenancy

> Verified: **increment A landed** — PATCH across sites/equips/points/boards and
> Get+Delete on widgets/sparks implemented on `rubix-gaps`, 2026-06-13, covered by
> `crates/rubix-server/tests/api_tests/{sites,points,widgets,sparks,boards}.rs`
> (both store backends). The ⟂ gates below are now green. Source:
> `rubix-server/src/api/{sites,equips,points,boards,widgets,sparks}`, the
> concrete + Postgres stores under `rubix-server/src/store/`, and the auth gate
> `rubix-server/src/auth/{gate,scope}.rs`. Design intent:
> [../../../docs/design/crud-and-tenancy.md](../../../docs/design/crud-and-tenancy.md).

Covers the **management surface** every other feature provisions through: the
full Create/Read/Update/Delete lifecycle of the domain entities (sites, equips,
points, boards, widgets, sparks), and the **org/site tenant boundary** that
keeps one tenant's rows invisible and untouchable to another. This is the
runbook for the "CRUD tenants and sites" work and the loop that verifies it.

Prereq: stack up per [../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md).
`$BASE`, `post()`, `del()` from
[../reference/API_CHEATSHEET.md](../reference/API_CHEATSHEET.md).

---

## What to prove

Two properties, each a section below.

1. **CRUD completeness.** Every domain entity round-trips through all four verbs.
   This runbook is also the **gap detector**: the backend today has **no Update
   verb on any entity** (verified across routers + both stores), and sparks/
   widgets lack Get/Delete. A gate that asks for `PATCH`/`PUT` is expected to
   **fail against current `main`** — that failure is the signal that drives the
   fix, not a doc bug. Each such gate is marked **⟂ gap** and links the design
   doc's increment.
2. **Tenant isolation.** A tenant is the `{org}/{site}` pair. A principal scoped
   to one org can neither **see** (reads filtered before the wire) nor **write**
   another org's sites; scoped SQL is structurally confined to its own rows. This
   must hold with auth **on** — it is a no-op with auth off (edge default).

> **Why this is one doc.** "CRUD tenants" and "CRUD sites" are the *same*
> operation: a tenant *is* a site (`UNIQUE(org, slug)`), `org` is the namespace
> string it carries. There is no separate tenant entity to manage.

---

## Runbook — Part A: CRUD completeness

### A1. Site lifecycle (the tenant)

```bash
# CREATE
SITE=$(post /api/v1/sites '{"org":"kfc","slug":"hq","display_name":"KFC HQ","tags":{"site":true}}' | jq -r .id)
# READ (list + get)
curl -s "$BASE/api/v1/sites?org=kfc" | jq '[.[].slug]'      # → ["hq"]
curl -s "$BASE/api/v1/sites/$SITE"   | jq '{org,slug,display_name}'
```

✅ Create returns `201` with a bare `Site`; list (filterable by `?org=`) and get
round-trip it.

✅ **Update (increment A — landed).** `PATCH /api/v1/sites/{id}` edits
`display_name`/`tags`:

```bash
curl -s -o /dev/null -w "patch-site %{http_code}\n" \
  -X PATCH "$BASE/api/v1/sites/$SITE" -H content-type:application/json \
  -d '{"display_name":"KFC Headquarters"}'            # → 200
```

✅ PATCH edits `display_name`/`tags`, leaves `org`/`slug`/`created_at`
untouched, and rejects a body carrying `org`/`slug` with **`400`** (they compose
the point keyexpr — see [POINTS_PRIORITY_ARRAY.md](POINTS_PRIORITY_ARRAY.md)).
Absent field = unchanged (partial PATCH).

```bash
# DELETE (cascades to equips/points/history/sparks — prove the blast radius first)
curl -s -o /dev/null -w "delete-site %{http_code}\n" -X DELETE "$BASE/api/v1/sites/$SITE"  # → 204
```

✅ Delete returns `204` and cascades. The admin surface must show child counts
**before** confirming — a delete here is destructive and irreversible.

### A2. Equip & Point lifecycle

```bash
SITE=$(post /api/v1/sites '{"org":"kfc","slug":"hq","display_name":"KFC HQ","tags":{"site":true}}' | jq -r .id)
EQUIP=$(post /api/v1/equips "$(jq -nc --arg s "$SITE" '{site_id:$s,path:"ahu-3",display_name:"AHU 3",tags:{"ahu":true}}')" | jq -r .id)
PT=$(post /api/v1/points "$(jq -nc --arg e "$EQUIP" '{equip_id:$e,slug:"sp",display_name:"SP",kind:"sp",unit:"°C",tags:{"sp":true}}')" | jq -r .point.id)
curl -s "$BASE/api/v1/equips/$EQUIP" | jq .display_name
curl -s "$BASE/api/v1/points/$PT"    | jq .point.slug
```

✅ Equip and point create/list/get/delete round-trip (delete proven in
POINTS_PRIORITY_ARRAY). Note point create wraps as `{keyexpr, point}` — id is
`.point.id`.

✅ **Update (increment A — landed).** `PATCH /api/v1/equips/{id}` and
`/api/v1/points/{id}` edit metadata only (`display_name`, `tags`, point
`unit`/`kind`); `equip.path` and `point.slug` are **immutable** (keyexpr
identity) — a body carrying them returns `400`. Both → `200`:

```bash
curl -s -o /dev/null -w "patch-equip %{http_code}\n" -X PATCH "$BASE/api/v1/equips/$EQUIP" -H content-type:application/json -d '{"display_name":"AHU 3 (north)"}'
curl -s -o /dev/null -w "patch-point %{http_code}\n" -X PATCH "$BASE/api/v1/points/$PT"    -H content-type:application/json -d '{"tags":{"sp":true,"trim":true}}'
```

### A3. Board, Widget, Spark surfaces

```bash
# Board has create/list/get/patch/delete (slug-addressed). PATCH edits
# display_name/enabled on the latest version; republishing the graph is a new POST.
curl -s "$BASE/api/v1/boards" | jq 'length'
curl -s -o /dev/null -w "patch-board %{http_code}\n" -X PATCH "$BASE/api/v1/boards/night-setback" \
  -H content-type:application/json -d '{"enabled":false}'   # → 200
# Widget + Spark now have create/list + get/delete (increment A).
curl -s "$BASE/api/v1/widgets" | jq 'length'
curl -s "$BASE/api/v1/sparks"  | jq 'length'
```

✅ **Thin surfaces (increment A — landed).** Widgets and sparks now have
`GET /{id}` and `DELETE /{id}`. The builder UI's widget **Remove** and spark
**get-by-id** triage are live:

```bash
WID=$(curl -s "$BASE/api/v1/widgets" | jq -r '.[0].id // empty')
[ -n "$WID" ] && curl -s -o /dev/null -w "get-widget %{http_code}\n"    "$BASE/api/v1/widgets/$WID"  # → 200
[ -n "$WID" ] && curl -s -o /dev/null -w "delete-widget %{http_code}\n" -X DELETE "$BASE/api/v1/widgets/$WID"  # → 204
SPK=$(curl -s "$BASE/api/v1/sparks" | jq -r '.[0].id // empty')
[ -n "$SPK" ] && curl -s -o /dev/null -w "get-spark %{http_code}\n"     "$BASE/api/v1/sparks/$SPK"   # → 200
```

### A4. Dashboards (many boards, org-overview or site-scoped)

A **dashboard** is a named board of widgets, owned by an `org`. It is either an
**org overview** (`site_id` absent — spans every site under the org) or
**site-scoped** (`site_id` set). Widgets belong to a dashboard via `dashboard_id`;
their `target` keyexpr already carries `org/site/...`, so an overview mixes tiles
from many sites freely. Source: `rubix-server/src/api/dashboards/`, the
`dashboards` table, and `rubix_core::Dashboard`.

```bash
SITE=$(post /api/v1/sites '{"org":"kfc","slug":"hq","display_name":"KFC HQ","tags":{"site":true}}' | jq -r .id)
# Org overview (no site_id) — the cross-site board.
OVR=$(post /api/v1/dashboards '{"org":"kfc","slug":"portfolio","title":"Portfolio"}' | jq -r .id)
# Site-scoped board.
SDB=$(post /api/v1/dashboards "$(jq -nc --arg s "$SITE" '{org:"kfc",site_id:$s,slug:"energy",title:"Energy"}')" | jq -r .id)

# List by org (overviews + every site's boards); filter to one site.
curl -s "$BASE/api/v1/dashboards?org=kfc"            | jq '[.[].slug]'   # → ["portfolio","energy"]
curl -s "$BASE/api/v1/dashboards?org=kfc&site_id=$SITE" | jq '[.[].slug]' # → ["energy"]

# PATCH the title (org/site/slug immutable); DELETE cascades to the board's tiles.
curl -s -o /dev/null -w "patch-dash %{http_code}\n"  -X PATCH "$BASE/api/v1/dashboards/$OVR" -d '{"title":"Portfolio Overview"}'  # → 200
curl -s -o /dev/null -w "delete-dash %{http_code}\n" -X DELETE "$BASE/api/v1/dashboards/$SDB"  # → 204
```

✅ Dashboards round-trip create/list(`?org=`,`?site_id=`)/get/patch/delete. A
slug is unique per scope (`(org,site)` for site boards, `(org)` for overviews).
✅ Widgets pin onto a dashboard: `POST /widgets` takes `dashboard_id`; omitting it
lands the tile on the site's **default** board (auto-created — the legacy
"pin to site" path, used by the agent `pin_widget` tool). List a board's tiles
with `GET /api/v1/widgets?dashboard_id=…`.
✅ Scope is auth-gated like sites: a site board uses `authorize_site_{read,write}`;
an overview uses the org scope. Reads a caller can't see are filtered pre-wire.
Coverage: `crates/rubix-server/tests/api_tests/dashboards.rs`.

> **Why a dashboard entity.** Before this, a widget hung directly off `site_id`,
> so there was exactly one implicit board per site and no cross-site overview.
> The `Dashboard` row makes "many boards, org or site" first-class — the UI
> `/builder` picks/creates them; an overview is the natural home for a
> portfolio view.

---

## Runbook — Part B: Tenant isolation

> **Requires auth ON.** Set `RUBIX_OIDC_*` (or use PATs) so the gate is live;
> with auth off every check is a no-op (edge default). Mint two scoped tokens via
> `POST /api/v1/tokens` — one scoped `org:"kfc"`, one `org:"bk"`. Reference:
> [AI_TOOLS_AND_AGENT.md](AI_TOOLS_AND_AGENT.md) for the tenancy rig and
> `crates/rubix-server/tests/api_tests/tenancy.rs`.

Seed two tenants, then assert mutual invisibility from each token.

```bash
# As an admin/global token: create one site per org.
post /api/v1/sites '{"org":"kfc","slug":"hq","display_name":"KFC HQ","tags":{"site":true}}'
post /api/v1/sites '{"org":"bk","slug":"hq","display_name":"BK HQ","tags":{"site":true}}'

# As the KFC-scoped token ($KFC):
curl -s "$BASE/api/v1/sites" -H "authorization: Bearer $KFC" | jq '[.[].org] | unique'   # → ["kfc"] only
```

✅ The KFC token's site list contains **only** `kfc` sites — `bk` is filtered
before the wire (`list_sites` → `authorize_site_read`). KFC never learns BK
exists.

✅ A KFC token writing a BK site (by id, if it somehow learned it) → `403`
(`authorize_site_write`: scope must cover the target + a write-capable role).

✅ Scoped SQL is structurally confined: a KFC-scoped `/query` session sees only
KFC rows even on `SELECT * FROM points_cur` (the views are pre-filtered, not
SQL-rewritten). Cross-tenant proven by
`crates/rubix-server/tests/api_tests/tenancy.rs` + `pg_query.rs`. See
[QUERY_AND_ROLLUP.md](QUERY_AND_ROLLUP.md).

✅ **Boundary is a prefix, not a string-prefix.** `kfc/hq` does **not** cover
`kfc/hq2` (sibling) — the same path-boundary match the bus and capabilities use.

---

## Acceptance criteria ("done")

- [ ] Site create/list(`?org=`)/get/delete round-trip; delete cascades.
- [ ] Equip + point create/list/get/delete round-trip.
- [ ] Board create/list/get/patch/delete round-trips.
- [x] **⟂→✅** `PATCH` edits metadata on site/equip/point/board; rejects identity-field
      (`org`/`slug`/`path`) changes with `400`. *(increment A — landed)*
- [x] **⟂→✅** Widget + spark gain Get + Delete. *(increment A — landed)*
- [ ] A scoped token's site list excludes other orgs (read filtered pre-wire).
- [ ] A scoped token writing another org's site → `403`.
- [ ] Scoped `/query` returns only the tenant's rows (structural view confinement).
- [ ] `org/site` boundary is a path boundary (`kfc/hq` ≠ `kfc/hq2`).

> **The former ⟂ gates are green.** Design increment **A** (Add Update across
> entities + fill the thin widget/spark surfaces) is implemented across both store
> backends and covered by `api_tests`. Identity fields (`org`/`slug`/`equip.path`/
> `point.slug`) remain immutable by design — a PATCH carrying one returns `400`.
> Increments **B** (first-class org/tenant surface) and **C** (named platform-admin
> principal) remain deferred per the design doc.

---

## Gotchas

- **Tenant ≠ a row.** There is no `tenants`/`orgs` table; `org` is a string on
  `Site` and on token scopes. "List tenants" = list sites grouped by `org`. A
  first-class org entity is design increment **B**, deferred until orgs need state
  of their own (suspend/billing) — don't build it ahead of that.
- **No platform-admin role.** The cross-org actor is a *global-scoped* (unscoped)
  Operator; that's "admin" today (increment **C**). Don't assume an `Admin` enum
  variant — it doesn't exist.
- **Identity fields are immutable for a reason.** `org`/`slug`/`equip.path`/
  `point.slug` compose the keyexpr every widget, history row, and tool target
  addresses by string. A rename is delete-and-recreate (cascade) or a future
  atomic "move," not a `PATCH`.
- **Isolation is RBAC over a shared store, not partitioning,** and only holds with
  auth on. The proof rig must enable auth or it proves nothing.

## Known issues / fixes

- **PATCH rejects identity fields with `400`, not `405`/`422`.** The handler
  accepts `org`/`slug`/`path`/point-`slug` in the body solely to reject them with
  a clear message (rather than silently ignoring an unknown field). A client that
  wants to rename does delete-and-recreate (cascade).
- **`unit` is set-only, not clearable.** Point PATCH uses `COALESCE`, so an absent
  `unit` is unchanged and a present one is set; there is no way to clear a unit
  back to null via PATCH today (delete-and-recreate if needed).
- **Board PATCH targets the latest version only** (`display_name`/`enabled`);
  changing `trigger`/`graph` is a new POST version, not a PATCH.
