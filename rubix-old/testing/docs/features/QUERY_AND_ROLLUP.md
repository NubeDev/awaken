# Feature — Query & History Rollup

> Verified: **verified** live on `rubix-gaps` (2026-06-13). All six gates green; one
> **backend gap found and fixed** — the HTTP `/query` route had no read-only guard
> (a `DROP` returned `{"rows":[]}` instead of refusing). Added `ensure_read_only` to
> the query engine so every path refuses writes. Source:
> `rubix-query/src/{context,sql,rollup}/`, `rubix-server/src/api/{query,his}`.

Covers: DataFusion read-only SQL over SQLite, the canonical tables + `points_cur`
view, `/his/rollup` time-bucketed aggregates, and tenant-scoped queries.

Prereq: stack up with `RUBIX_QUERY=1` (default) and some history (run the sim a
while, or `POST .../cur` a few values). `$BASE`, `post()` from the cheatsheet.

---

## What to prove

1. Canonical tables resolve by bare name and join.
2. `points_cur` exposes the resolved keyexpr + effective value.
3. Read-only enforcement: `SELECT`/`WITH` only.
4. Rollup buckets by interval × aggregate, epoch-aligned.
5. Scoped queries read only their `{org}/{site}`.

---

## Runbook

### 1. Canonical tables + join

```bash
post /api/v1/query '{"sql":"SELECT count(*) AS n FROM points"}' | jq
post /api/v1/query '{"sql":"SELECT s.org, s.slug, count(p.id) AS points FROM sites s JOIN equips e ON e.site_id=s.id JOIN points p ON p.equip_id=e.id GROUP BY 1,2"}' | jq
```

✅ Bare-name `points` resolves; the three-table join returns per-site point counts.
(Schema is read live from `PRAGMA table_info`, so even empty tables resolve columns.)

### 2. points_cur view

```bash
post /api/v1/query '{"sql":"SELECT keyexpr, cur_value, cur_ts FROM points_cur WHERE cur_value IS NOT NULL"}' | jq
```

✅ Returns each point's `{org}/{site}/{equip}/{point}` keyexpr alongside its
effective `cur_value` — the dashboard-ready flattened surface.

### 3. Read-only enforcement

```bash
post /api/v1/query '{"sql":"DROP TABLE his"}' | jq      # → 400 "only a single read-only SELECT/WITH..."
post /api/v1/query '{"sql":"INSERT INTO points VALUES (1)"}' | jq
post /api/v1/query '{"sql":"SELECT 1; SELECT 2"}' | jq
```

✅ All three rejected with `400 {"error":"only a single read-only SELECT/WITH
statement is allowed"}` — only a **single** `SELECT`/`WITH` statement runs; DDL/DML
and multi-statement are refused. This guard (`rubix_query::ensure_read_only`) runs in
the engine itself, so the HTTP route **and** the agent `query` tool are both covered.
(Before the fix the HTTP route had no guard: a `DROP` returned `{"rows":[]}` — a
no-op against the read-only DataFusion providers, so no data was ever lost, but it
read like success. See "Known issues / fixes".)

### 4. Rollup

```bash
post /api/v1/his/rollup "$(jq -nc --arg p "$POINT" '{points:[$p],interval:"five_minute",agg:"avg"}')" | jq
post /api/v1/his/rollup "$(jq -nc --arg p "$POINT" '{points:[$p],interval:"hour",agg:"max",start:"2026-06-13T00:00:00Z",end:"2026-06-14T00:00:00Z"}')" | jq
```

✅ The five-minute `avg` returns epoch-aligned buckets; the bounded `hour`/`max`
respects the `start`/`end` window. Valid intervals: `minute`, `five_minute`,
`fifteen_minute`, `hour`, `day`, `week`. Aggregates: `avg|min|max|sum|count|first|last`.
Response is `{"series":[{point_id, bucket, value, samples}]}` (one row per
point×bucket). An unknown interval/agg is rejected at deserialization (`422`).

### 5. SQL-injection guard on point ids

```bash
post /api/v1/his/rollup '{"points":["'\''; DROP TABLE his; --"],"interval":"hour","agg":"avg"}' | jq
```

✅ Rejected — point ids carrying a quote/NUL are refused before reaching SQL
(`QueryScope`/rollup validation). The only caller-controlled SQL is the point-id
literal, and it's validated.

### 6. Tenant scope (if exercising the scoped path)

A scoped query (via the agent `query` tool, or a scoped HTTP principal) reads
through tenant-filtered views.

✅ A query scoped to `nube/hq` returns rows only under that prefix; a sibling
`nube/hq2`'s rows are invisible. Unscoped/edge-no-auth is global by design.

---

## Acceptance criteria ("done")

- [x] Bare-name tables resolve and join; empty tables still expose columns.
- [x] `points_cur` returns keyexpr + effective value.
- [x] DDL/DML and multi-statement are rejected (`400`, guard now in the engine).
- [x] Rollup produces epoch-aligned buckets for each interval × aggregate.
- [x] Point-id injection is refused (`illegal literal`).
- [x] Scoped queries read only their `{org}/{site}` (`rubix-query/tests/scoped.rs`, 5/5).

---

## Gotchas

- **`/query` is SQLite-only** for `sites`/`equips`/`points`/`sparks`. `his` can
  union a Parquet cold tier when `RUBIX_HIS_PARQUET` is set; Postgres federation is
  a cloud-feature option (`QueryEngine::open_postgres`).
- A `503` from `/query` means `RUBIX_QUERY=0`, not a missing route.
- If `db_state.txt` (direct SQLite) has rows but `/query` returns 0, the bug is in
  the query layer (view/schema/scope), not the write path — TRIAGE §4.

## Known issues / fixes

Verified live 2026-06-13. Five of six gates were green as-scaffolded; Gate 3 surfaced
a **real backend gap, now fixed**:

**Backend fix — `/query` had no read-only guard.** The HTTP `/query` route called
`QueryEngine::query` directly, which passed any SQL to `ctx.sql()` with no
statement-type check. The DataFusion providers are read-only, so a `DROP TABLE
his`/`INSERT` could not mutate the store (verified: `his` row count unchanged after a
`DROP`) — but the route *accepted* them: `DROP` returned `{"rows":[]}` (looked like
success) and `INSERT` returned an obscure planning error, neither a clean refusal.
The agent `query` tool already guarded (`is_read_only`), but the HTTP route did not —
the scaffold's "same guard the tool uses" was not true of the route.

Fix: added `rubix_query::ensure_read_only(sql)` and call it at the top of both
`QueryEngine::query` and `scoped_query`, so **every** caller (HTTP route and agent
tool) refuses non-`SELECT`/`WITH` and multi-statement input with
`QueryError::NotReadOnly` → `400 "only a single read-only SELECT/WITH statement is
allowed"`. Unit-tested in `rubix-query` (`sql::run::read_only_tests`, accepts
SELECT/WITH, rejects INSERT/UPDATE/DELETE/DROP/multi-statement); confirmed live
(DROP/INSERT/UPDATE/multi → `400`; SELECT/WITH still return rows). Full `rubix-query`
suite green (lib 6, query 6, rollup 5, scoped 5, his_tier 4) — no regression. The
agent tool's own `is_read_only` is left in place as a second, decoupled layer
(`rubix-tools` deliberately does not depend on `rubix-query`).

**Doc note:** `/his/rollup` response is `{"series":[{point_id, bucket, value,
samples}]}`; `points_cur.cur_value` comes back as a string (`"19.0"`).
