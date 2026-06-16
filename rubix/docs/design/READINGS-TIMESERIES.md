# READINGS-TIMESERIES — a first-class time-series plane on the rubix substrate

Design for the **readings / history** model: how high-volume, append-only
time-series (meter readings, sensor samples, any "value at an instant") are
stored, written, queried, rolled up, and aged out — as a **distinct plane**, not
as ordinary `record`s. The document/config side (collections, typed CRUD,
realtime) is [BACKEND-COLLECTIONS.md](BACKEND-COLLECTIONS.md); the chart/query
surface that consumes readings is [DASHBOARDS-SCOPE.md](DASHBOARDS-SCOPE.md). The
**scope authority** is [SCOPE.md](../SCOPE.md) — specifically its two-plane split
("Data plane … append-only, edge-owned" vs "Config plane … the real conflict
surface"), its **historian boundary** in the Scale path, and open questions 2/3.
Where this doc and SCOPE.md disagree, SCOPE.md wins.

## Thesis

A reading is not a document. The collection layer ([BACKEND-COLLECTIONS.md](BACKEND-COLLECTIONS.md))
gives every `kind` a typed, validated, audited, undoable, realtime CRUD record —
exactly right for a *site*, a *meter*, a *dashboard*. It is exactly **wrong** for a
meter reading, of which there are millions, which is never edited, never undone,
and whose only interesting query is "this series, this window, bucketed." Treating
readings as `record`s forces the high-volume append-only plane through the
low-volume audited config plane and pays the config plane's costs (audit row,
undo capture, correlation id, full-table-scan list) on every sample.

SCOPE already draws this line — it just isn't drawn in the store yet. This doc
draws it: a **dedicated `reading` table**, a real **measurement-time column**, the
**indexes** a time-series read needs, a **write path that is the data plane** (not
the command gate), and a **historian boundary** so the read side can swap raw rows
for pre-aggregated rollups without a chart noticing. The store stays SurrealDB and
stays generic (principle 4 is preserved — see "Generic vs. structural" below); the
domain is still not baked in. What changes is that *time-series-ness* — series +
instant + value — becomes a structural primitive the engine can index, the same
way `record` is a primitive today.

## Where it is today (the delta)

Grounded in the current code, not the design docs:

| Concern | Today | Delta to design here |
| --- | --- | --- |
| Storage | readings are `record`s with `content.kind:"history"` in the **one** `record` table ([list.rs:44](../../crates/rubix-core/src/record/list.rs#L44) `WHERE content.kind = $kind`) | **a dedicated `reading` table**, data-plane, lean rows |
| Measurement time | stuffed in `content.ts` as an **ISO string** ([history.mjs:76](../../../nhp/seed/history.mjs#L76)); unindexed | **a native `datetime` `at` column**, indexed |
| Query time basis | the query layer buckets on the row's **`created`** (write time), not `content.ts` ([series.rs:84](../../crates/rubix-query/src/aggregate/series.rs#L84)) | **bucket on `at`** — fixes a latent trend-collapse bug |
| Indexes | **none** — `init_schema` defines `record/tag/tagged` SCHEMALESS, zero `DEFINE INDEX` ([init_schema.rs:19](../../crates/rubix-store/src/init_schema.rs#L19)) | **`DEFINE INDEX` on `(namespace, series, at)`** for the scoped range-per-series read |
| Permissions | `record`/`tag`/`tagged` get explicit row perms ([define.rs:59](../../crates/rubix-gate/src/permission/define.rs#L59)); a new table with none is invisible-or-leaky on a scoped session | **`reading` gets the same scoped `FOR select` + writes `NONE`** overwrite |
| Write path | the seed writes readings **through the command gate** (`POST /records` → `apply()`, [portfolio.mjs:198](../../../nhp/seed/portfolio.mjs#L198)) — audited, undo-captured, correlated per sample | **append-only data-plane write**, never the gate |
| Ingest | `rubix-ingest` already bypasses the gate but appends to the **`record`** table with no `at` ([append.rs:36](../../crates/rubix-ingest/src/persist/append.rs#L36)) | **ingest appends to `reading`**, mapping `at`/`series`/`value` |
| Read shape | UI pulls the **entire** history collection and filters client-side ([batch.ts:83](../../../nhp/ui/src/features/dashboards/query/batch.ts#L83) `useAllHistory()`) | **windowed, series-scoped** read via the historian |
| Rollups / scale | none; every chart re-scans raw rows | **pre-aggregated rollup tables** behind a **historian boundary** (SCOPE Scale path) |
| Per-row payload | `quantity`/`unit` repeated on **every** sample ([history.mjs:71](../../../nhp/seed/history.mjs#L71)) | **on the series**, not the sample — lean rows |

Two of these are correctness, not just performance:

- **The trend-collapse bug.** The DataFusion path buckets on `created`. The seed
  back-dates samples via `content.ts` but lets the gate stamp `created = time::now()`
  ([persist.rs:34](../../crates/rubix-gate/src/command/persist.rs#L34)), so all 48
  hours of a series share one `created` ≈ seed time. The NHP UI hides this because
  it reads raw rows and re-filters on `content.ts` itself ([batch.ts:95](../../../nhp/ui/src/features/dashboards/query/batch.ts#L95)) — but the
  moment a board moves onto `/query` (the [DASHBOARDS-SCOPE.md](DASHBOARDS-SCOPE.md)
  §5 plan), the trend flattens to a point. Measurement time must be the time the
  query layer buckets on. That is the `at` column.
- **The plane violation.** Routing readings through `apply()` means each sample
  produces an audit row and an undo entry. SCOPE: "undo covers definitions/config
  only — never the data plane (readings, insight firings)." Readings must not enter
  the undo stack or the command gate at all.

## Generic vs. structural — why this does not break principle 4

SCOPE principle 4 is "no fixed domain schema (equipment / site / point); structure
comes from tagging." A `reading` table does **not** bake in a domain. `series`,
`at`, `value` are not an ontology — they are the *time-series primitive*, the same
way `record { id, namespace, content, created }` is the *document primitive* already
declared in [init_schema.rs:19](../../crates/rubix-store/src/init_schema.rs#L19).
"Voltage", "site", "phase L1" stay where they belong: on the **series definition**
(the register/point record on the config plane), reached through `series`. The
table is **`SCHEMALESS` with a few `DEFINE FIELD`/`DEFINE INDEX`** declarations —
extra fields still pass through untyped (principle 4 intact), but the three columns
a time-series read lives or dies on are typed and indexed. This is the same move
BACKEND-COLLECTIONS makes for `content.kind` (load-bearing convention, not a baked
ontology), one level deeper.

## The model — a `reading` table keyed by series and instant

The DDL splits across the two seams that already own it: **table existence +
fields + indexes** in `rubix-store`'s `init_schema`
([init_schema.rs:19](../../crates/rubix-store/src/init_schema.rs#L19), a
store-boundary concern), and the **row-level `PERMISSIONS`** in `rubix-gate`'s
`define_gate_schema` on the root handle
([define.rs:59](../../crates/rubix-gate/src/permission/define.rs#L59), where the
`record`/`tag`/`tagged` perms already live). Shown together here for legibility:

```surql
-- store seam (init_schema): existence, structural fields, indexes
DEFINE TABLE   IF NOT EXISTS reading SCHEMALESS;          -- data plane, append-only
DEFINE FIELD   IF NOT EXISTS series ON reading TYPE record;          -- the series this sample belongs to
DEFINE FIELD   IF NOT EXISTS at     ON reading TYPE datetime;        -- MEASUREMENT instant (not write time)
DEFINE FIELD   IF NOT EXISTS value  ON reading TYPE number;          -- the numeric sample
DEFINE FIELD   IF NOT EXISTS namespace ON reading TYPE string;       -- edge partition (SCOPE: edge-owned)
DEFINE FIELD   IF NOT EXISTS created ON reading TYPE datetime DEFAULT time::now();  -- RECEIVE time, explicit (see below)
-- Anything else (raw quality flags, source key) lands in free-form `content`.

-- The hot read is "this namespace, this series, this window" — so the index is
-- namespace-aware, not just (series, at), to serve scoped reads and per-edge
-- retention without scanning other tenants' partitions.
DEFINE INDEX  IF NOT EXISTS reading_ns_series_at ON reading FIELDS namespace, series, at;  -- the hot query
DEFINE INDEX  IF NOT EXISTS reading_ns_at        ON reading FIELDS namespace, at;          -- cross-series scans + retention sweep

-- gate seam (define_gate_schema, root handle): scope reads, forbid scoped writes
DEFINE TABLE OVERWRITE reading SCHEMALESS
  PERMISSIONS
    FOR select WHERE namespace = $auth.namespace    -- same scoping as `record` (define.rs:61)
    FOR create, update, delete NONE;                -- appends go through the root/owner handle, never a scoped session
```

Design decisions, each load-bearing:

- **Scoped permissions are not optional — they are correctness.** A table with no
  `PERMISSIONS` is invisible-or-leaky on a scoped session; the only reason reads are
  namespace-safe today is the explicit `FOR select WHERE namespace = $auth.namespace`
  the gate overwrites onto `record`/`tag`/`tagged`
  ([define.rs:59-70](../../crates/rubix-gate/src/permission/define.rs#L59)). `reading`
  gets the identical treatment: scoped `select`, and `create/update/delete NONE` so
  no principal can write through its scoped session. The append helpers run on the
  **root/owner store handle** *after* the capability check (the same posture as the
  gate's own writers) — the per-request capability decision is the gate; the write
  itself is a direct owner-side append, never a scoped-session write.
- **`series` is a reference to the series-defining record, not a denormalized
  blob.** In NHP a series *is* a `history:true` register
  ([meter-types.mjs:31](../../../nhp/seed/meter-types.mjs#L31)); `series` points at
  that register's id. `unit`, `quantity`, `precision`, `chart_group` are read from
  the register, **once**, not copied onto every sample. This deletes the per-row
  duplication the seed has today ([history.mjs:71-77](../../../nhp/seed/history.mjs#L71)).
  Caveat: this is right for *display* metadata, but if the **physical meaning** of
  historical samples ever changes (a recalibrated scale, a unit base change), the
  series definition needs **versioning or immutable unit semantics** so old samples
  keep their original meaning — a one-edit unit change must not silently reinterpret
  history (open question 8).
- **`at` ≠ `created`, and `created` is set explicitly.** `at` is when the world
  produced the value; `created` is when we received/persisted it. There is **no
  automatic row stamp** in this codebase — `Record::new` and the gate mutation both
  set timestamps explicitly with `time::now()`
  ([persist.rs:35](../../crates/rubix-gate/src/command/persist.rs#L35)). So the
  append path either sets receive-time `created`/`updated` itself or relies on the
  `DEFAULT time::now()` declared above; it must never let `created` stand in for
  `at`. The query layer **buckets on `at`** — closing the trend-collapse bug at the
  root.
- **Append identity is deterministic, so re-append and sync-replay are both
  no-ops.** `(series, at)` is the natural key. Rather than a non-unique index plus
  hope, the row id is **derived deterministically** from `(series, at)` (e.g.
  `reading:⟨hash(series, at)⟩`). This makes a re-seed/backfill an idempotent upsert
  *and* — because `rubix-sync` dedups by **id** ([ship.rs:88](../../crates/rubix-sync/src/data/ship.rs#L88))
  — makes a replayed shipped reading a no-op for free, with no new dedup mechanism. A
  `UNIQUE` composite index is the simpler alternative if a deterministic id proves
  awkward; decide in open question 5. Either way, idempotency is **enforced**, not
  assumed.
- **The indexes are namespace-first.** Every chart query is "namespace N, series X,
  `at` between t0 and t1, ordered by `at`." `reading_ns_series_at` serves exactly
  that with a range scan, not a full-table filter — **provided the read path issues a
  filtered SurrealQL statement** (see the read-path caveat below; the index does
  nothing for a blind `SELECT *`). `reading_ns_at` serves cross-series time scans and
  the cheap per-edge retention sweep (`DELETE … WHERE namespace = $ns AND at < cutoff`).
- **Rows stay lean.** `{ series, at, value }` + namespace is the whole hot path.
  Free-form `content` remains available (principle 4) for the rare sample that
  carries quality flags — but the common reading is four fields.

### Why a separate table, not a `kind:"reading"` collection

This is the one place we deliberately *don't* follow "everything is a record"
([BACKEND-COLLECTIONS.md](BACKEND-COLLECTIONS.md)). The justification is SCOPE's own
plane split: collections govern the **config/document plane** (typed, validated,
audited, undoable, realtime-per-row). Readings are the **data plane** (append-only,
edge-partitioned, never undone, queried in bulk by window). A separate table buys
the three things a shared table cannot: its **own indexes** (config records don't
want a `(series, at)` index), its **own retention/rollup lifecycle**, and a **scan
that never wades through config records**. It is the concrete form of SCOPE's
"single historian boundary so [a TSDB] swap is contained."

## Write path — the data plane, never the command gate

Readings append; they do not command. Two writers, one table, both gate-bypassing
by design (the capability decision is taken once, up front — not per sample):

- **Ingest (real sources).** `rubix-ingest` already authorizes the Zenoh key-space
  once at subscribe and then appends without re-crossing the gate
  ([append.rs:11](../../crates/rubix-ingest/src/persist/append.rs#L11)). The change:
  it appends to **`reading`**, mapping the decoded `Sample`
  ([sample.rs:20](../../crates/rubix-ingest/src/subscribe/sample.rs#L20)) →
  `{ series, at, value }`. `at` comes from the sample payload when the source
  stamps it, else from arrival time; `series` is resolved from the key-space →
  series mapping; extras stay in `content`.
- **Bulk append endpoint (`POST /readings`).** For the seed, backfills, and any
  non-Zenoh source: a batch append that writes straight to `reading` on the
  principal's **edge partition** (namespace), append-only, **no `apply()`**. It
  takes a capability check **once per request** (a new fail-closed `readings:append`
  grant — open question 1), not per sample, mirroring the ingest contract. This is
  the path the seed switches to (below); it is *not* `POST /records`.

Neither path produces audit/undo per sample — consistent with SCOPE ("readings …
never undone"). Lineage for readings is the correlation id stamped at **ingest**
(SCOPE: "minted at the gate (principal actions) **or at ingest** (data)"), carried
on the bus event the append publishes, not an audit row per write.

## Read path — bucket on `at`, behind a historian boundary

The query layer gets a **`reading` canonical table** with real columns, so charts
filter and bucket on typed fields instead of `json_get`-ing into `content`:

```rust
// rubix-query: add to CanonicalTable (provider/schema.rs:24)
CanonicalTable::Readings => "reading",
// arrow_schema: series Utf8, at Timestamp(µs), value Float64, + id/namespace/created
```

- **Bucketing uses `at`**, not `created` — the root fix for the trend-collapse bug.
  But this is **a new adapter path, not an in-place switch.** Today
  `aggregate/series.rs` reads `created` and `content.<field>` from generic `record`
  rows ([series.rs:85](../../crates/rubix-query/src/aggregate/series.rs#L85)); blindly
  repointing it at `at`/`value` would break every existing record-backed reading and
  rule fixture until writers/migration land. So **`Records` and `Readings` stay
  separate adapter paths** — the `Readings` adapter buckets on `at`/`value`, the
  legacy `Records` adapter is untouched — and callers move onto the historian seam
  incrementally. The bug is fixed *for the readings plane* without destabilising the
  record plane.
- **The index win needs a filtered SurrealQL path — it is not free.** ⚠ The current
  provider does `SELECT * FROM <table>` into an in-memory `MemTable` and lets
  DataFusion plan over that ([scan.rs:34](../../crates/rubix-query/src/provider/scan.rs#L34)).
  A DataFusion predicate over an already-materialised table **does not touch the
  SurrealDB index** — the whole table is pulled to memory first. To realise the
  `reading_ns_series_at` range-scan win, the historian must issue a **direct filtered
  SurrealQL statement** (`SELECT … WHERE namespace = $ns AND series = $s AND at
  BETWEEN $t0 AND $t1 ORDER BY at`) — or `rubix-query` must grow a real
  predicate-pushdown `TableProvider`. The filtered-SurrealQL historian path is the
  smaller, recommended first move; the pushdown provider is the general fix if/when
  charts need arbitrary SQL over readings.
- **The historian boundary.** A single read seam — "give me namespace N, series X,
  window [t0,t1], grain G" — decides raw-vs-rollup behind the chart's back and is
  *where the filtered SurrealQL lives*. Below a threshold it range-scans `reading`;
  above it, it reads the rollup table. This is the SCOPE Scale path ("All history
  access stays behind a single historian boundary so [a TSDB] swap is contained, not
  a refactor") made concrete, and where DASHBOARDS-SCOPE §1's "time bucketing →
  Backend" lands for readings.

### Rollups — the first scale lever, designed now, built when volume bites

```surql
DEFINE TABLE IF NOT EXISTS reading_rollup SCHEMALESS;
DEFINE FIELD namespace ON reading_rollup TYPE string;  -- edge partition, same scoping as reading
DEFINE FIELD series    ON reading_rollup TYPE record;
DEFINE FIELD grain     ON reading_rollup TYPE string;  -- minute|hour|day, aligned to rubix-query Grain
DEFINE FIELD start     ON reading_rollup TYPE datetime; -- epoch-aligned bucket start
DEFINE FIELD agg       ON reading_rollup TYPE object;   -- { avg, min, max, last, sum, count }
DEFINE INDEX reading_rollup_ns_series_grain_start ON reading_rollup FIELDS namespace, series, grain, start;
-- + the same scoped PERMISSIONS overwrite as `reading` (select WHERE namespace = $auth.namespace; writes NONE)
```

Rollups are **pre-aggregated on a short schedule** (not on the hot append path —
SCOPE OQ2 argues against per-sample fold at high rate), epoch-aligned to
`rubix-query`'s existing `Grain` so a chart bucket and a rule bucket line up exactly
(the alignment DASHBOARDS-SCOPE §5 requires). The historian reads the coarsest
rollup that satisfies the requested grain, falling back to raw `reading` for the
live tail. SCOPE's external-TSDB escape hatch (TimescaleDB as a DataFusion
`TableProvider`) plugs in **behind this same boundary** if pre-aggregation stops
being enough — no chart change.

**Rollup repair is part of the rollup design, not a footnote.** Because `at` is
measurement time, a late or out-of-order sample lands in a bucket that may already
be rolled up. A rollup is only authoritative if it can be **recomputed for a dirty
bucket**: the fold must track which `(namespace, series, grain, start)` buckets
received a sample since their last computation and recompute those, or recompute a
trailing window each cycle. This ties to the lateness policy (open question 6) — the
two must be decided together, before rollups become the read path's source of truth.

## Retention & edge↔cloud sync

- **Retention is a cheap sweep.** Append-only + `reading_ns_at` index makes
  `DELETE FROM reading WHERE namespace = $ns AND at < $cutoff` a range delete, not a
  scan. Raw rows age out; rollups persist longer (configurable per grain). This is
  the relief valve for SCOPE's single-engine write/pub-sub concentration bet (OQ2) on
  the readings axis. The primitive is landed —
  [`sweep_readings_before`](../../crates/rubix-core/src/reading/sweep.rs) (per
  namespace, exclusive `at` cutoff, off the gate, returns the deleted count). The
  *policy surface* and the schedule that drives it remain open (OQ9).
- **Sync reuses the data-plane *model* but needs its own codec/apply path.** The
  *partitioning* carries over unchanged — readings are edge-partitioned (`namespace`
  = edge identity, [append.rs:7](../../crates/rubix-ingest/src/persist/append.rs#L7)),
  so "two edges never write the same row, reconciliation is ordering + dedup, not
  merge" still holds, and the deterministic `(series, at)` id makes the existing
  id-dedup apply for free. But `rubix-sync` is **`Record`-specific end to end** —
  `WireRecord`, `encode_record`, `apply_record`, the `ship_key` builder
  ([ship.rs:20-152](../../crates/rubix-sync/src/data/ship.rs#L20)) all hard-code the
  record shape. Shipping readings therefore needs **either a `WireReading` +
  `apply_reading` sibling path, or a table-discriminated data-plane envelope** that
  carries the target table alongside the row. It does **not** apply unchanged. Rollups
  are derived, so they either ship as data or recompute cloud-side from shipped raw
  rows — open question 4. Large raw-blob shipping is out of scope (readings are small
  scalars, not the blob path BACKEND-COLLECTIONS file-fields worry about).

## Migration from `kind:"history"`

Existing `kind:"history"` records in the `record` table are migrated once into
`reading`: read each, map `content.ts → at`, `content.register → series`,
`content.value → value`, insert into `reading`, delete the old record. For NHP the
cheaper path is **re-seed** (the seed is idempotent and the synthetic history has no
production value). For any real captured history a one-shot migration command
(idempotent by the deterministic `(series, at)` id, so a re-run or a crash mid-migration
re-lands the same rows) is the safe path. Either way the `record` table is left
holding only config/document records afterward — the plane split is clean. **Built:**
[`migrate_history_to_readings`](../../crates/rubix-core/src/reading/migrate.rs)
(append-then-delete per record, malformed records skipped and left in place),
exposed as the one-shot `--migrate-history` server flag.

## Seed changes (concrete)

The seed is the stand-in poller (it "plays the poller and writes time-series so
dashboards have a trend", [history.mjs:1](../../../nhp/seed/history.mjs#L1)). It
moves onto the data plane:

1. **Write via the bulk append, not the gate.** [portfolio.mjs:198-206](../../../nhp/seed/portfolio.mjs#L198)
   stops calling `createRecord(row)` (→ `POST /records` → `apply()`) and calls a new
   `appendReadings(series, samples)` (→ `POST /readings`). No audit/undo per sample.
2. **Emit `{ at, value }`, drop the duplicated metadata.** [history.mjs:70-78](../../../nhp/seed/history.mjs#L70)
   stops repeating `kind`/`meter`/`register`/`quantity`/`unit` on every row. A
   sample becomes `{ at: ts.toISOString(), value }`; the `series` (register id) is
   passed once per batch. `quantity`/`unit` already live on the register
   ([meter-types.mjs:30](../../../nhp/seed/meter-types.mjs#L30)) — the chart reads
   them from there.
3. **Idempotent re-seed by `(series, at)`.** Readings have no `key` to dedupe on
   today, so the seed back-fills only freshly-created registers ([portfolio.mjs:197](../../../nhp/seed/portfolio.mjs#L197)).
   With `(series, at)` as the natural identity, the append can upsert-by-(series,at)
   or the seed can clear the series' window first — either makes re-seed clean
   instead of "only when the register is new."
4. **Checks follow the field move.** `check.mjs` / `records-check.mjs` assert on
   `at`/`series`/`value` against the `reading` table (via the historian read or a
   `kind`-free list), not `content.ts`/`content.register` against `record`.

## UI changes (concrete)

Yes, the UI needs changes — the current model only scales because the seed is tiny.

1. **Stop fetching the whole collection.** [batch.ts:83](../../../nhp/ui/src/features/dashboards/query/batch.ts#L83)
   `useAllHistory()` (pulls *every* history row) and `useMeterHistory` (filters
   client-side) are replaced by a **windowed, series-scoped** query through the
   historian: `useSeriesHistory(series, { from, to, grain })`. At real volume the
   current "fetch all, filter in JS" is the thing that falls over first.
2. **Read `at`, not `content.ts`.** The `HistorySample` interface ([batch.ts:34](../../../nhp/ui/src/features/dashboards/query/batch.ts#L34))
   loses the repeated `quantity`/`unit`/`meter`/`register` (now on the series) and
   keys the x-axis on `at`.
3. **Series-scoped joins.** [site-board.ts:79](../../../nhp/ui/src/features/dashboards/auto-build/site-board.ts#L79)
   joins history to a register by `content.meter`+`content.register`; with `series`
   = register id this becomes a direct `series === register.id` join, no string
   splitting.

These align the UI with the DASHBOARDS-SCOPE direction (backend buckets, frontend
presents) instead of fighting it.

## Contracts honored

- **SCOPE two-plane split (data vs config).** Readings become the data plane in the
  store, not a config-plane record. Undo/audit/realtime-per-row stay on the config
  plane where SCOPE puts them.
- **Two enforcement points (STACK-DEISGN #1).** Reads stay on the scoped SurrealDB
  session, confined by the **explicit `reading` row permission** (`FOR select WHERE
  namespace = $auth.namespace`, mirroring [define.rs:61](../../crates/rubix-gate/src/permission/define.rs#L61))
  plus the namespace-first index — the table is *not* safe without that overwrite.
  The append path takes its capability decision **once** (subscribe for ingest,
  per-request for bulk) and then writes on the **root/owner handle**, exactly as
  `rubix-ingest` already justifies ([append.rs:11](../../crates/rubix-ingest/src/persist/append.rs#L11))
  — it does not introduce a new per-sample gate, and scoped sessions can never write
  (`FOR create, update, delete NONE`).
- **One shared `Reading` domain type.** `series`/`at`/`value`/`namespace`/`created`/
  `content` is defined **once** as a small Rust type used by ingest, the bulk-append
  endpoint, sync (`WireReading`), migration, and the query projection — so the shape
  cannot drift across the five places that touch it. This is the readings analogue of
  `rubix-core`'s `Record`, and it is the cheapest insurance against the seams above
  diverging.
- **Generic-by-construction (SCOPE principle 4).** `series/at/value` is the
  time-series primitive, not a domain ontology; the table is SCHEMALESS with typed
  indexed columns; domain meaning stays on the series record. No domain is baked
  into the binary.
- **SurrealDB does as much as possible (#6).** The hot query is an index range scan;
  rollups are SurrealQL aggregates; retention is a range delete. Only the
  optional-TSDB swap leaves SurrealDB, and only behind the historian seam.
- **Correlation id.** Readings carry the **ingest-minted** correlation id on their
  bus event (SCOPE: minted "at the gate … or at ingest"), preserving trace pivots
  without an audit row per sample.

## Build order (smallest load-bearing first)

1. **`reading` table + namespace-first indexes + `at`/`series`/`value`** in
   `init_schema` ([init_schema.rs:19](../../crates/rubix-store/src/init_schema.rs#L19)),
   **and its scoped `PERMISSIONS` in `define_gate_schema`** on the root handle
   ([define.rs:59](../../crates/rubix-gate/src/permission/define.rs#L59)) — the two
   land together or the table is unsafe. Plus the shared `Reading` domain type.
   Additive: nothing reads it yet.
2. **A separate `Readings` adapter in `rubix-query`** ([schema.rs:24](../../crates/rubix-query/src/provider/schema.rs#L24))
   that buckets on `at`/`value`, **leaving the legacy `Records` reading path
   untouched** ([series.rs:85](../../crates/rubix-query/src/aggregate/series.rs#L85)).
   Fixes the trend-collapse bug for the readings plane without destabilising
   record-backed fixtures. (The index *win* arrives with the filtered-SurrealQL
   historian in step 6, not here.)
3. **Bulk append endpoint `POST /readings`** (data-plane write on the owner handle,
   capability-once, deterministic `(series,at)` id) + point `rubix-ingest` at
   `reading` ([append.rs:36](../../crates/rubix-ingest/src/persist/append.rs#L36)) +
   the `WireReading`/`apply_reading` sync path ([ship.rs:20](../../crates/rubix-sync/src/data/ship.rs#L20)).
4. **Seed onto the append path** + drop duplicated metadata + idempotent re-seed via
   the deterministic id (seed changes above).
5. **UI windowed historian read** (UI changes above) — retires `useAllHistory`.
6. **Historian boundary (filtered SurrealQL) + rollup tables + rollup repair** — the
   scale lever *and* where the index range-scan win is finally realised; raw-only is
   correct until volume warrants rollups, so this is last and independently shippable.

Step 1 is additive. Step 2 fixes the correctness bug on a *new* adapter, not by
mutating the record path. Steps 3–5 move readings off the command gate (write path +
sync + seed + UI together, since they share the `Reading` type and id scheme). Step 6
is the scale lever and the index payoff, built when real cardinality (SCOPE OQ3) says
raw scans stop being enough.

## Open questions

1. **Append capability.** Does bulk `POST /readings` need a new fail-closed
   `Capability` variant (`readings:append`), or is it admitted under the namespace
   scope like an ordinary data-plane write? It is gate-bypassing either way, so the
   decision is *where the once-per-request check lives*, mirroring AGENT/ingest's
   memory-write question. (Permissions themselves are **decided**, not open: scoped
   `select`, writes `NONE`, appends on the owner handle — see "The model".)
2. **`series` identity & cardinality.** `series` = the register/point record id
   directly, or an interned compact series id (cheaper index, extra indirection)?
   At fleet scale the index width on `series` matters; decide before the index ships.
3. **Read-path pushdown.** Realise the index range-scan via a **direct filtered
   SurrealQL** historian query (smaller, recommended) or a real **predicate-pushdown
   `TableProvider`** in `rubix-query` (general, larger)? The current `SELECT *`-into-
   `MemTable` provider ([scan.rs:34](../../crates/rubix-query/src/provider/scan.rs#L34))
   gives **no** index benefit, so one of these is required, not optional.
4. **Rollup trigger.** Pre-aggregate **on write** (cost on the hot append path) vs.
   a **short scheduled fold** (simpler, bounded lag) vs. **lazy on first read**
   (cache-fill). SCOPE OQ2 (write/pub-sub concentration) argues against on-write fold
   at high rate — likely scheduled.
5. **Append/dedup identity policy.** **Deterministic `(series, at)` row id**
   (idempotent upsert + free sync-replay dedup, recommended) vs. a **`UNIQUE`
   composite index** vs. **explicit duplicate handling** with a source sequence /
   quality field. Decide before any writer lands — it shapes the id scheme, the seed,
   and sync.
6. **`at` source & lateness.** When a source doesn't stamp the sample, `at` =
   arrival time — but late/out-of-order samples then bucket "now". Define the policy:
   require source `at`, or accept arrival-time with a lateness bound, and how
   out-of-order samples drive **rollup repair** (the dirty-bucket recompute above).
7. **Rollup sync.** Do rollups **ship** edge→cloud as data, or **recompute**
   cloud-side from shipped raw rows? Recompute keeps the sync stream lean (raw only)
   but doubles compute; shipping doubles the stream. Tie-in with the `WireReading`
   data-plane envelope (`rubix-sync` is `Record`-specific today, [ship.rs:20](../../crates/rubix-sync/src/data/ship.rs#L20)).
8. **Series unit-semantics versioning.** Reading `unit`/`scale` from the series
   record is right for display, but a **recalibration or unit-base change** must not
   silently reinterpret historical samples. Does the series definition need a
   version, or immutable physical-unit semantics with display-only overrides? Affects
   the migration and the "one-edit unit correction" claim.
9. **Retention policy surface.** Per-series vs. per-namespace vs. global TTL; raw vs.
   rollup retention split; whether retention is config-plane (a record) or a system
   setting. Edge devices with small disks make this not optional.
10. **Migration of real captured history.** For any non-synthetic `kind:"history"`
    already written, confirm the one-shot id-idempotent migration command and whether
    it runs edge-side, cloud-side, or both.
