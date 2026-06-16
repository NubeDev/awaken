# Readings / time-series plane — follow-ups

Tracking work deliberately deferred from the readings/time-series pass
(`rubix/docs/design/READINGS-TIMESERIES.md`), decisions made where the spec left
room, and one environment-blocked verification step. Each item is shaped so it
slots into the historian seam landed here without a refactor.

## Out of scope this pass (noted, not built)

- **Rollup tables (`reading_rollup`) + the scheduled fold + rollup repair.** The
  read path is raw-`reading` range scans, which is correct until volume warrants
  pre-aggregation (SCOPE OQ3, "verify at real cardinality"). The historian read
  (`read_readings_window` / `GET /readings`) is the single seam a rollup reader
  slots behind — below a threshold it range-scans `reading`, above it it would
  read the coarsest satisfying rollup. The DDL sketch and the dirty-bucket repair
  policy are in the design doc; nothing here precludes them. (Doc "Rollups".)
- **Rollup sync.** No rollups ship or recompute cloud-side yet (design OQ7). Raw
  readings already ship via the `WireReading` path; rollups would either ship as
  data or recompute from the shipped raw rows behind the same seam.
- **Retention sweeper — *policy/scheduling* still deferred (primitive now built).**
  The range-delete primitive `sweep_readings_before(db, namespace, cutoff)` is
  landed in `rubix-core` (`reading/sweep.rs`): a per-partition, `at`-bounded
  `DELETE FROM reading WHERE namespace = $ns AND at < $cutoff` off the gate,
  returning the deleted count, with integration tests. What is **still** deferred
  is the *policy surface* (design OQ9: per-series vs per-namespace vs global TTL,
  the cutoff values) and the **schedule/job** that drives it — a caller fans the
  primitive over the partitions it owns; the primitive itself takes no policy.
- **Generic predicate-pushdown `TableProvider`.** Per the decision, the historian
  read is a direct filtered SurrealQL statement (`WHERE series = $s AND at BETWEEN
  $t0 AND $t1 ORDER BY at`), which the `(namespace, series, at)` index serves as a
  range scan. A general DataFusion pushdown provider (so arbitrary SQL over
  `reading` hits the index, not a `SELECT *`-into-`MemTable`) is the larger,
  later move (design OQ3). The `Readings` query adapter added here still scans the
  whole table into a `MemTable` for ad-hoc `/query` SQL — fine at seed scale.
- **Series unit-semantics versioning** (design OQ8). `unit`/`quantity` are read
  from the series record for display; a recalibration / unit-base change could
  silently reinterpret historical samples. Needs a version or immutable
  physical-unit semantics. Untouched.
- ~~**Real captured-history migration command.**~~ **Built** (design "Migration",
  OQ10). `migrate_history_to_readings` in `rubix-core` (`reading/migrate.rs`) maps
  every `kind:"history"` record (`content.ts → at`, `content.register → series`,
  `content.value → value`) into the `reading` plane and deletes the source record;
  id-idempotent (deterministic `(series, at)`), append-then-delete so a crash
  re-runs cleanly, and defensive (a malformed record is skipped and left in place,
  never appended as junk). Exposed as a one-shot `--migrate-history` server flag
  that runs and exits without binding a socket. For NHP the cheaper path is still
  a re-seed (synthetic history, no production value); this is the safe path for
  any *real* captured history.

## Decisions made where the spec left room

- **`series` SurrealDB type.** Stored as a `record` link (`series ON reading TYPE
  record`, per the doc DDL), pointing at the register record in the `record`
  table. The domain `Reading.series` and all read projections expose the **bare**
  register id (decoded through the typed `RecordId`, avoiding SurrealDB's
  bracket-quoting) so a board joins by `series === register.id` directly.
- **Deterministic id hash.** `reading:<uuid_v5(namespace, "{series}|{at_rfc3339}")>`
  — UUIDv5 (SHA-1) over the canonical `series|at` string. Canonical RFC3339
  rendering collapses `…:00Z` and `…:00.000Z` (same instant) to one id, so a
  re-seed in either notation is idempotent. (`v5` feature added to the workspace
  `uuid` dep.)
- **Idempotent append.** `INSERT INTO reading $rows ON DUPLICATE KEY UPDATE …`
  with `created` omitted from the update branch, so a re-append overwrites the
  mutable fields but preserves the original receive-time `created`
  (`DEFAULT time::now()` stamps it only on first insert).
- **`readings-append` capability.** A new fail-closed `Capability` variant,
  distinct from `ingest-publish`: ingest authorizes the Zenoh stream once at
  subscribe; `readings-append` authorizes the non-Zenoh bulk endpoint
  (seed/backfills) once per request. The demo seed cast (`operator`/`admin`) now
  holds it.
- **Ingest `Sample → Reading` mapping.** `value` from `content.value` (a sample
  without one is rejected — not a well-formed reading); `series` from
  `content.series` else the Zenoh key's last segment; `at` from `content.at`/`ts`
  else arrival time; promoted keys stripped from the retained `content` so rows
  stay lean.
- **Sync envelope.** A `WireReading` sibling path on a table-discriminated
  key-space (`rubix/sync/data/reading`), rather than retrofitting a
  table-discriminated envelope onto the Record path. `order.rs` (`in_apply_order`,
  `Record`-typed) was left untouched; reading batches are ordered inline by
  `(created, id)`.

## Done since the readings pass

- **The Rust `--seed-dev` demo seed now writes into the `reading` data plane.**
  Previously it wrote `kind:"reading"` records into the `record` table through the
  command gate. `seed/history.rs` now builds lean [`Reading`]s keyed off the bare
  point id (`series`, `at`, `value`; display metadata stays on the point record),
  and `seed/portfolio.rs` appends each point's trailing window via
  `append_readings` off the gate — mirroring the NHP seed and the live
  ingest/append path. Re-seed stays idempotent (deterministic `(series, at)` ids),
  and the per-reading tag-graph attach is dropped (readings are not records).

## Environment-blocked verification (NOT a code issue)

- **Live end-to-end seed run** could not be executed in this sandbox. The rubix
  server **builds, boots, and runs `--seed-dev` successfully** (it provisions
  `acme_operator` with the `readings-append` grant — confirmed in its boot log),
  but its bound listener is **not reachable over loopback** from the test
  harness: `127.0.0.1:8097/8099` are forwarded to a pre-existing host server
  running *older* code (its `/readings` route 404s), and a server bound on a fresh
  port (8088/8091) gets connection-refused from any other shell invocation.
  - **Mitigation:** the endpoint is instead verified by an **in-process HTTP
    integration test** (`rubix/crates/rubix-server/tests/http/readings/`) that
    drives the real router via `tower::oneshot` — covering `POST /readings`
    (append, idempotent re-append with no duplicate rows, `403` without the grant,
    `400` on a malformed timestamp) and `GET /readings` (at-ordered windowed read
    on the scoped session). The seed `.mjs` files all pass `node --check`.
  - **To run live** (on a machine without the forwarding quirk): boot
    `RUBIX_BIND=127.0.0.1:8097 cargo run --bin rubix-server -- --nhp --seed-dev`
    against a fresh `RUBIX_DATA_DIR`, then `RUBIX_BASE=http://127.0.0.1:8097 node
    nhp/seed/seed.mjs && node nhp/seed/check.mjs && node nhp/seed/records-check.mjs`.
