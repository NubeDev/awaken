# Flow tag-based point selection — scope & gaps

Working notes for letting flow board nodes select points by **Haystack tag** instead
of a single hardcoded keyexpr, so a board can say "write to every point tagged `sp`"
or "evaluate every point tagged `temp`" rather than naming each point. Captures
(1) how point selection works today and why this is a gap, (2) the design choices a
fresh session must make, (3) the staged scope, and (4) exactly what and how to test.

> **For a session picking this up cold:** read this top to bottom, then re-grep the
> cited `file:line`s on your first action (code drifts; treat anything here as
> unverified until you confirm it). The testing contract lives in
> [`testing/docs`](../../../testing/docs) — this doc tells you which runbook to extend.

Source crates: [`rubix-flow`](../../../crates/rubix-flow) (the engine wrapper + node
impls + the `PointAccess` seam) and [`rubix-server`](../../../crates/rubix-server)
(`flow/` host impl, `store/`, `api/boards/`). UI under
[`rubix/ui`](../../../ui) `src/features/flows`.

Related: [flow-runtime-redesign.md](flow-runtime-redesign.md) (the seam this extends),
[RULES_ENGINE.md](../../../testing/docs/features/RULES_ENGINE.md) and
[BOARDS_REFLOW.md](../../../testing/docs/features/BOARDS_REFLOW.md) (the runbooks this
adds gates to).

---

## 1. How point selection works today (and the gap)

Every point-touching flow node takes **one** `point` keyexpr from config and acts on
exactly that point. There is **no tag-based selection anywhere in the flow runtime.**

Code-grounded (re-grep before trusting):

- **`read_point`** — `config_str(context, "point")`,
  [`node/read_point/mod.rs:33`](../../../crates/rubix-flow/src/node/read_point/mod.rs).
- **`query_his`** — `config_str(context, "point")`,
  [`node/query_his/mod.rs:33`](../../../crates/rubix-flow/src/node/query_his/mod.rs).
- **`write_point`** — `config_str(context, "point")`,
  [`node/write_point/mod.rs`](../../../crates/rubix-flow/src/node/write_point/mod.rs)
  (the `write` fn).
- **`emit_spark`** — gained an optional `point`/`points` config for the implicated
  point on 2026-06-14 (still keyexpr, not tags),
  [`node/emit_spark/mod.rs`](../../../crates/rubix-flow/src/node/emit_spark/mod.rs).
- **Config schema** — every one is a single `ConfigField::new("point", …, Keyexpr,
  true)`,
  [`board/component_schema.rs:214,229,249`](../../../crates/rubix-flow/src/board/component_schema.rs).

The **seam** (`PointAccess`,
[`port.rs:100`](../../../crates/rubix-flow/src/port.rs)) is entirely keyexpr-addressed:
`read_point(keyexpr)`, `write_point(keyexpr, …)`, `query_his(keyexpr, limit)`,
`watch(prefix)`. **No method takes tags.**

### The infrastructure that already exists (reuse it — do not reinvent)

- **Tag filtering on the store** —
  `Store::list_points(equip_id, site_id, tags: &[String])`,
  [`store/points.rs:46`](../../../crates/rubix-server/src/store/points.rs), which calls
  `filter_tags(points, tags, |p| &p.tags)` (`store/codec.rs`). Semantics are **`has_all`
  marker-presence** (a point matches when it carries *every* requested tag), the same as
  the HTTP `?tags=a,b` filter proven in
  [POINTS_PRIORITY_ARRAY.md](../../../testing/docs/features/POINTS_PRIORITY_ARRAY.md) §2.
- **Tag query parse** — `parse_tags` (comma-separated),
  [`api/tag_query.rs`](../../../crates/rubix-server/src/api/tag_query.rs).
- **Keyexpr build** — `Point::keyexpr(org, site_slug, equip_path, point_slug)`,
  [`rubix-core/src/model.rs:67`](../../../crates/rubix-core/src/model.rs); the store can
  list `(id, keyexpr)` pairs (`all_point_keys`,
  [`store/keyexpr.rs:107`](../../../crates/rubix-server/src/store/keyexpr.rs)).
- **Option-source dropdowns** — a `ConfigField` with `.with_option_source("points")`
  renders as an editor dropdown fetched from `GET /boards/options/{source}`,
  [`api/boards/options.rs:67`](../../../crates/rubix-server/src/api/boards/options.rs).

So the host already knows how to turn `(scope, tags)` into a set of matching points
with their keyexprs. The gap is purely that the **flow seam and nodes never ask it to.**

---

## 2. Design choices the session must make

### 2a. Scope: how does a tag-only node know *which* tenant to search?

A board derives its tenant `{org}/{site}` from its keyexpr-bearing configs
([`board/tenant.rs:13`](../../../crates/rubix-flow/src/board/tenant.rs):
`KEYEXPR_CONFIG_KEYS = ["point", "site"]`). A node that selects by **tags has no
keyexpr**, so there is nothing to derive org/site from, and a global tag scan across
all tenants is wrong (a cross-tenant leak and unbounded).

**Decision (recommended):** add a required `scope` config field holding an
`{org}/{site}` prefix (the same shape `emit_spark.site` uses). Then:

- The node passes `(scope, tags)` to the new seam method.
- Add `"scope"` to `KEYEXPR_CONFIG_KEYS` so `tenant_org()`/`tenant_site()` keep working
  for a board whose only tenant signal is a tag node's scope.
- `scope` gets `.with_option_source("sites")` so the editor offers a site dropdown
  (the source already exists, `options.rs:72`).

This keeps tenancy explicit and fail-closed (no scope → the node errors, never scans
globally), and is consistent with how `emit_spark` already names a site.

### 2b. Seam shape: one resolution method, nodes stay thin

Add **one** method to `PointAccess` rather than tag-variants of every method:

```rust
/// Resolve the keyexprs of all points in `scope` ({org}/{site}) carrying every
/// tag in `tags` (has_all). Empty `tags` is a scope-wide match. Fail-closed:
/// an unparseable scope or unknown site is an Err, never a silent empty set
/// (distinguish "no points matched" — Ok(vec![]) — from "scope invalid").
async fn resolve_points_by_tags(
    &self,
    scope: &str,
    tags: &[String],
) -> Result<Vec<String>, FlowAccessError>;
```

Default impl on the trait returns `Err(Unsupported(...))` (the established fail-closed
pattern — see `emit_spark`/`watch` defaults, `port.rs`), so test fakes and the agent's
own board access need not implement it. The server's `StorePointAccess`
([`flow/access.rs:40`](../../../crates/rubix-server/src/flow/access.rs)) implements it
over `list_points` + `Point::keyexpr`, run on the blocking pool like the other store
calls (`on_store(...)`).

Then each node, **when `tags` config is present**, calls
`resolve_points_by_tags(scope, tags)` and **fans out** over the result, reusing its
existing single-keyexpr logic per resolved point. When `point` is present instead, it
behaves exactly as today (back-compat). Exactly one of `point` / `tags` is set — both
or neither is a config error (mirror `rule`'s `source_config`,
[`node/rule/mod.rs:155`](../../../crates/rubix-flow/src/node/rule/mod.rs)).

### 2c. Fan-out semantics per node (the real design work)

- **`write_point` (tags):** resolve → write the same value/priority to **each** matched
  point. Coalescing is already per-actor-state keyed; with fan-out the last-command
  cache must be **keyed per resolved keyexpr** (today it stores one `LAST_COMMAND`,
  `node/write_point/mod.rs`). Output: one `output` tick (or a small summary); decide and
  document.
- **`query_his` / `read_point` (tags):** resolving to N points but emitting **one**
  `output` is ambiguous. Two viable models — **pick one and write it down**:
  1. **Per-point fan-out into the graph** (a rule node runs once per point). Cleanest
     mental model, but reflow edges are static — fanning a dynamic N into one downstream
     `rule` node needs the engine to deliver N messages on the same edge (verify reflow
     supports repeated sends on one outport within a tick; the engine already reads
     interior links from the `MessageSent` stream — see flow-runtime-redesign.md §"Observing
     link values").
  2. **One combined frame** — `query_his` concatenates all matched points' history into
     one frame with an added `keyexpr`/`point` column, and the rule folds across points.
     This changes the rule frame shape (today it is two columns `ts,value`,
     [`node/rule/frame.rs`](../../../crates/rubix-flow/src/node/rule/frame.rs)); a
     per-point group is the Haystack-idiomatic shape but is a bigger change.

  **Recommended for v1:** model (1) for `write_point` (fan-out is natural for a sink),
  and for the rules board use model (1) at the **board level** — one `query_his`+`rule`
  +`emit_spark` triple per point is *not* required; instead a single tag-`query_his`
  feeding one `rule` that the engine re-runs per delivered frame. If reflow cannot
  redeliver on one edge within a tick, fall back to **per-point board expansion** done by
  the host at load time (expand a tag node into N concrete nodes before building the
  reflow `Network`) — note this in the doc and the runbook.

> This ambiguity is the crux. Spend the first hour proving which fan-out reflow actually
> supports with a throwaway board before writing node code; record the answer in the
> runbook's "Gotchas".

### 2d. Config schema + UI

- Add a `tags` `ConfigField` to `read_point`/`query_his`/`write_point` (and update the
  `emit_spark` implicated-point story if a finding should implicate all matched points).
  There is **no list/array `FieldType`** today
  ([`component_schema.rs:15`](../../../crates/rubix-flow/src/board/component_schema.rs) —
  variants are String/Keyexpr/Integer/Number/Boolean/Enum/Json). Options: reuse
  `FieldType::String` with comma-separated tags (parse via `parse_tags`, matches the HTTP
  convention) — **recommended, smallest change** — or add a `Tags`/`StringList` variant
  (touches the UI editor's field renderer,
  [`ui/src/features/flows/components/node-config-form.tsx`](../../../ui/src/features/flows/components/node-config-form.tsx)).
- A new `option_source` (e.g. `"tags"`) listing the tags in scope would make the editor a
  multi-select; optional for v1 (free-text comma list works first).

### 2e. Non-goals for v1

- No tag *expressions* (AND/OR/NOT) — only `has_all` (what the store does today).
- No cross-tenant / cross-site tag selection — `scope` is required and single-site.
- No change to `agent_call`/`datasource` selection.

---

## 3. Staged scope (suggested commit breakdown)

Each stage builds + tests green before the next (the project's implementation cycle).

1. **Seam + host resolution.** Add `resolve_points_by_tags` to `PointAccess`
   (`port.rs`) with a fail-closed default; implement on `StorePointAccess`
   (`flow/access.rs`) over `list_points` + `Point::keyexpr`. Unit-test the host impl
   (matching, `has_all`, empty-match Ok, bad-scope Err). **No node changes yet.**
2. **`write_point` by tags (the generator sink).** Add `tags`+`scope` config, fan-out
   write, per-keyexpr coalescing. This is the smallest end-to-end node and unblocks the
   generator board. Add the config-schema field.
3. **`query_his` (+ `read_point`) by tags.** Implement the fan-out model chosen in §2c.
   This unblocks the rules board.
4. **Config schema + UI field.** Expose `tags`/`scope` in the palette
   (`component_schema.rs`) and the editor (`node-config-form.tsx`), with the `sites`
   option source on `scope`.
5. **The two demo boards** (the acceptance artifact — see §4): a tag-driven generator
   board and a tag-driven 5-rule spark board, stored + interval, visible in Flow Boards.

---

## 4. What and how to test

Tests live in the suite at [`testing/docs`](../../../testing/docs). **Extend the existing
runbooks; do not start a parallel doc.** The operating contract (boot → run gates →
capture-on-fail → record) is in [`testing/docs/README.md`](../../../testing/docs/README.md).

### Library / unit (each stage)

- **Host resolution** (`crates/rubix-server` unit or `flow/access.rs` tests): a seeded
  topology with mixed tags; assert `resolve_points_by_tags("nube/hq", ["sp"])` returns
  exactly the `sp` keyexprs, `["sp","setpoint"]` is `has_all`, an absent tag → `Ok(vec![])`,
  a bad scope → `Err`. Mirror the HTTP `?tags=` cases already proven in
  [POINTS_PRIORITY_ARRAY.md](../../../testing/docs/features/POINTS_PRIORITY_ARRAY.md) §2.
- **Node fan-out** (`crates/rubix-flow/tests/board.rs`, in-process with a fake
  `PointAccess` that implements `resolve_points_by_tags`): a tag `write_point` commands
  every matched point; a tag `query_his` delivers the chosen fan-out shape; `point` vs
  `tags` mutual-exclusion errors. The fake is the existing `board.rs` test harness.

### Live, end-to-end (the headline gates)

Add an **L5** section to
[RULES_ENGINE.md](../../../testing/docs/features/RULES_ENGINE.md) "Runbook (live gates)"
and a **tag** gate to
[BOARDS_REFLOW.md](../../../testing/docs/features/BOARDS_REFLOW.md) §4, and a script
`testing/scripts/run-tag-boards.sh` modeled on the existing
`testing/scripts/run-rules.sh` (git-ignored; boot → seed → assert → leave-stack-up).
The script must:

1. **Boot a clean stack** (`make build-be`; clean `rubix.db`; `make dev-be`; wait
   `/healthz`). `RUBIX_ZENOH=0` is fine — history is HTTP-seeded.
2. **Provision a TAGGED topology** (the whole point — *no hardcoded ids downstream*):
   - writable points tagged `sp` (e.g. `ahu-3/sp1`, `ahu-4/sp2`, `kind:"sp"`,
     `tags:{"sp":true}`) for the generator to write;
   - sensor points tagged `temp` (e.g. `ahu-3/temp`, `ahu-4/temp`, `kind:"sensor"`,
     `tags:{"temp":true}`) seeded with deterministic history (reuse `run-rules.sh`'s
     `his_series` 60s-spaced batches via `POST /points/{id}/his`).
3. **Store + run the GENERATOR board** (interval): `trigger` → `write_point{scope:"nube/hq",
   tags:["sp"]}`. Gate: after a few intervals **every `sp` point's `cur_value` /
   `priority_array` slot updates** — assert by listing `?tags=sp` and checking each, and
   that adding a *new* `sp` point makes it picked up next tick (proves it's tag-driven,
   not a frozen id list). Use `POST /boards` then `GET /boards/{slug}/outputs` /
   `GET /points?tags=sp`.
4. **Store + run the 5-RULE SPARK board** (interval, one board): tag `query_his{scope,
   tags:["temp"]}` → 5 `rule` nodes (reuse the stored rules `temp-high`, `temp-low`,
   `temp-spike-zscore`, `rate-of-change`, `stuck-sensor`) → `emit_spark` each. Gate: the
   board generates the expected sparks **across all `temp` points**, each spark carrying
   its implicated point (the 2026-06-14 fix — `point_ids` non-empty). Assert via
   `GET /api/v1/sparks?rule=<tag>` and `select(.point_ids|length>0)`.
5. **Tenancy gate:** a `temp` point under a *different* site is **not** matched by
   `scope:"nube/hq"` (fail-closed scope; reuse S4 reasoning in
   [scenarios/README.md](../../../testing/docs/scenarios/README.md)).
6. **Leave the stack up** and print the Flow Boards URL
   (`http://127.0.0.1:5180/o/nube/s/hq/flows` — confirm the route) and the Sparks URL so
   the boards are inspectable in the UI, exactly as `run-rules.sh` does.

### Acceptance ("done")

- [ ] `resolve_points_by_tags` host impl: `has_all`, empty-match `Ok([])`, bad-scope
      `Err` (unit).
- [ ] `write_point` by tags fans out to every matched point; per-keyexpr coalescing
      holds (no history spam on steady state).
- [ ] `query_his` by tags evaluates every matched point (chosen fan-out model
      documented in BOARDS_REFLOW Gotchas).
- [ ] `point` and `tags` are mutually exclusive; missing both / both set → node error.
- [ ] Config schema exposes `tags`+`scope`; the editor renders them (UI gate).
- [ ] Generator board (stored, interval) writes all `sp` points by tag, picks up a
      newly-added `sp` point — **no hardcoded ids**.
- [ ] 5-rule spark board (stored, interval) generates sparks across all `temp` points,
      each spark carrying its implicated point.
- [ ] A point outside `scope` is not matched (tenancy fail-closed).
- [ ] Two boards visible + inspectable in the Flow Boards UI; stack left up.

---

## 5. Known traps / things that will bite

- **Tag-only nodes break `tenant_org()`** unless you add `"scope"` to
  `KEYEXPR_CONFIG_KEYS` (`board/tenant.rs:13`) — otherwise a rules board whose only
  tenant signal is a tag node has **no derivable org**, and stored-rule resolution +
  org-scoped composition fail closed (you'll see "no rule store"/resolve errors).
- **reflow dynamic fan-out is unproven** — §2c is the gamble. Validate it with a
  throwaway board first; if one edge can't redeliver N frames in a tick, expand tag nodes
  to N concrete nodes at `load` time (`board/load.rs`) and document that the fan-out is
  load-time, not runtime.
- **`write_point` coalescing is single-slot today** — fan-out without per-keyexpr keying
  would make point B's command suppress point A's. Key the last-command cache by keyexpr.
- **`has_all` is presence, not value** — markers match on key presence (`{"sp":true}`),
  consistent with the rest of rubix; don't invent value matching.
- **Sensor vs writable** — the generator writes, so its points must be `kind:"sp"`/`"cmd"`
  (a `sensor` rejects `/write` with 400, POINTS_PRIORITY_ARRAY §6). Tag the *right* kind.
- **The 2026-06-14 spark `point_ids` fix** (sparks now carry their implicated point) is a
  prerequisite for the rules-board gate to be meaningful — confirm it's still in
  (`flow/access.rs` `emit_spark` resolves `draft.points`); see
  [RULES_ENGINE.md](../../../testing/docs/features/RULES_ENGINE.md) "Known issues / fixes".

---

## 6. Current state at the time of writing (2026-06-14)

A live stack is up with 10 stored rules (`run-rules.sh`) on `rubix.db`, all generating
sparks with implicated points. The three rule-chaining patterns are characterized
(in-script composition ✅, board fan-out ✅ with a found one-shot routing quirk under
investigation, rule→rule piping ✅-rejected-by-design). **None of that uses tags** —
that gap is this doc's scope. No tag selection code exists yet; this is greenfield on
the seam.
