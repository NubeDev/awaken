# Feature — External Datasources (read-only SQL passthrough)

> Verified: **library-verified** on `rubix-gaps` (2026-06-13). Core engine built
> and unit-tested in isolation (`cargo test -p rubix-datasource` → 38 unit pass,
> 5 live-DB tests `#[ignore]`d); `cargo clippy` clean; `unsafe_code = forbid`.
> **Not yet wired into a running stack** — there is no HTTP route, spark node, or
> AI tool over it, so there are no live-stack gates here. Integration is owned by
> a separate session. Source: `crates/rubix-datasource/`. Design:
> [docs/design/datasources.md](../../../docs/design/datasources.md).

Covers: reading external SQL databases (primarily TimescaleDB/Postgres) by
running **operator-authored native SQL** and returning `{ columns, rows }` — the
same shape `rubix-query` returns — **without** copying the data into the rubix
store and **without** routing it through DataFusion. The external engine plans
and executes; rubix only passes SQL down and reads rows back, under caps, over a
read-only role.

This is distinct from `rubix-query` (DataFusion over rubix's *own* SQLite/Postgres
tables) and from `drivers.json` protocol drivers (which *write* live point data
*into* the store). A datasource is a connection rubix *reads out of* at query time.

Prereq for the live gates only: a throwaway Postgres/Timescale and the
`RUBIX_DS_TEST_*` env — see [Live-DB gates](#live-db-gates-ignored-by-default).
The unit gates need nothing but the workspace.

---

## What to prove

1. **Read shape.** A read returns `{ columns, rows }` with `serde_json` cell
   values and columns in SQL select-list order — render-compatible with `rubix-query`.
2. **Bound parameters only.** Values bind as `$1`-style, never spliced into SQL;
   a SQL-shaped string passed as a param is returned verbatim, never executed.
3. **One statement per call.** Multi-statement input is refused before it reaches
   the backend, so a `SELECT`-only role can't be bypassed by a trailing statement.
4. **Caps.** Row/byte caps truncate the result and set a `breached` flag;
   wall-clock is enforced as the engine's native `statement_timeout`.
5. **Breach is the caller's policy, not baked in.** The lenient (dashboard) path
   reads the truncated result; the strict (spark) path turns the same breach into
   a hard `CapBreached` error.
6. **Two entry points, one path.** Raw operator SQL and named-query invocation run
   through the *same* validation + caps; the AI tier invokes named queries only.
7. **Credentials are registry-owned.** Callers pass only a datasource id; the
   decrypted password lives only in the pool and never logs (`Debug` redacts it).
8. **Schema discovery.** `describe` returns an operator-declared schema blob if the
   manifest carries one, else introspects `information_schema` under the read-only role.

---

## Runbook (unit gates — no live DB)

Run from `rubix/`. These exercise every cap/parameter/single-statement/named-query
/describe code path against a fake `SqlBackend`, so no database is needed.

```bash
cargo test  -p rubix-datasource          # 38 pass, 5 live-DB ignored
cargo clippy -p rubix-datasource --all-targets   # clean
```

### 1. Read shape + column order

`backend::postgres::tests::shape_preserves_column_order_from_first_row` and
`shape_fills_missing_keys_with_null`.

✅ Rows come back as ordered JSON cell lists; column order follows the SQL
select-list (the `row_to_json` path; `serde_json` `preserve_order` keeps it). A
missing key in a later row fills `null`. `ResultSet` serializes to
`{ columns:[{name,type_name}], rows:[[…]], breached:bool }`.

### 2. Bound parameters only

`statement::bind::tests::*` and `executor::run::tests::execute_passes_validated_sql_params_and_bounds`.

✅ `Param` (`Null|Bool|Int|Float|Text|Timestamp`) round-trips through JSON and is
handed to the backend as a typed, positional list — never concatenated into SQL.
A `Param::Text("'; DROP TABLE t; --")` is just data. (Live proof that the engine
treats it as data: `parameter_is_data_not_sql`, below.)

### 3. One statement per call

`statement::single::tests::*` — eight cases.

✅ `SELECT 1` and a single trailing `;` pass; `SELECT 1; DROP TABLE t` is refused
with `MultiStatement`; empty input → `EmptyStatement`. A `;` inside a string
literal, dollar-quote (`$$…$$` / `$tag$…$tag$`), or `--` / `/* */` comment is **not**
treated as a separator. `executor::run::tests::execute_rejects_multi_statement_before_backend`
confirms the backend is **never called** on a multi-statement input.

### 4. Caps truncate + flag

`caps::admit::tests::*` and `executor::cap::tests::truncates_at_row_cap_and_flags_breach`.

✅ Row and byte axes are independent and all-or-nothing per row; crossing either
sets `breached` and stops collection. `unbounded()` never breaches. The executor
also passes the backend a `fetch_bound = max_rows + 1` so the pull itself is bounded
*and* a breach is detectable (`…passes_validated_sql_params_and_bounds` asserts the
bound is `row_cap + 1` and the wall-clock is forwarded).

### 5. Lenient vs strict breach

`executor::cap::tests::strict_path_errors_on_breach` /
`executor::run::tests::strict_path_turns_breach_into_error`.

✅ `execute`/`invoke_named` return a possibly-truncated `ResultSet` (dashboard reads
`breached`); `Executor::strict(result)` turns a breach into
`DatasourceError::CapBreached { rows, bytes }` (the spark path — a spark must error
on partial data, never fold a truncated grid into a finding). The policy is the
caller's; neither path is baked into the executor.

### 6. Two entry points, one path

`executor::run::tests::invoke_named_*`.

✅ `invoke_named` resolves the operator-authored SQL by name and runs it through the
same `execute` path; an unknown name → `UnknownQuery`; wrong arity → `ParamCount`.
The caller of a named query supplies only the name + bound params, never SQL text
(the AI tier's trust model — operator authors the SQL, the AI parameterizes it).

### 7. Registry / unknown id

`registry::run::tests::unknown_datasource_errors`.

✅ `executor(id)` / `describe(id)` on an unknown id → `UnknownDatasource`.
`backend::postgres::tests::debug_redacts_password` proves `PostgresConn`'s `Debug`
prints `<redacted>`, never the secret.

### 8. Schema discovery

`describe::run::tests::declared_blob_short_circuits_introspection` and
`introspection_groups_columns_by_qualified_table`.

✅ A manifest-declared `SchemaBlob` short-circuits live introspection; otherwise
`information_schema.columns` rows fold into `{ tables:[{name, columns:[{name,type_name}]}] }`
grouped by qualified `schema.table`.

---

## Live-DB gates (`#[ignore]`d by default)

The SQL-touching path (real binding, `statement_timeout`, `row_to_json` shaping,
`information_schema`) is covered by `tests/live_postgres.rs`, marked `#[ignore]`
because this environment has no Postgres. To run:

```bash
docker run --rm -e POSTGRES_PASSWORD=pw -p 5433:5432 timescale/timescaledb:latest-pg16
RUBIX_DS_TEST_HOST=localhost RUBIX_DS_TEST_PORT=5433 \
  RUBIX_DS_TEST_DB=postgres RUBIX_DS_TEST_USER=postgres RUBIX_DS_TEST_PASSWORD=pw \
  cargo test -p rubix-datasource --test live_postgres -- --ignored
```

Gates: `binds_parameters_and_shapes_rows`, `parameter_is_data_not_sql` (a
SQL-shaped param round-trips as data), `statement_timeout_kills_runaway`
(`pg_sleep(10)` under a 200 ms cap → `Backend` error), `registry_executes_and_caps_through_the_pool`
(named query truncated at the row cap on the lenient path, the same breach → error
on the strict path), and `describe_introspects_information_schema`.

> A `SELECT`-only role is the production write guard. The live tests use the
> superuser for setup convenience and assert read behavior only.

---

## Acceptance criteria ("done" for the core engine)

- [x] `{ columns, rows }` shape, column order preserved, JSON cells.
- [x] Bound `$1`-style parameters only; no textual interpolation path exists.
- [x] Multi-statement refused before the backend is touched.
- [x] Row/byte caps truncate + flag; wall-clock via native `statement_timeout`.
- [x] Breach surfaced both as a `breached` flag (lenient) and a `CapBreached` error (strict).
- [x] Raw-SQL and named-query entry points over one executor; AI tier = named only.
- [x] Registry owns creds + one small pool per id; password never logged.
- [x] `describe` = declared blob or `information_schema` introspection.
- [x] `unsafe_code = forbid`; clippy clean; 38 unit tests green; 5 live `#[ignore]`.
- [ ] **Integration** — not in scope here. No HTTP route / spark node / AI tool /
      `datasources.json` loaded by a running binary yet (owned by another session).

---

## Gotchas

- **sqlx, not the workspace `postgres` crate.** This crate uses sqlx as its single
  Postgres client stack (matching nexus). To avoid the `sqlx` facade pulling
  `sqlx-sqlite` → `libsqlite3-sys 0.30`, which collides with `rusqlite`'s
  `libsqlite3-sys 0.35` (only one crate may link `sqlite3`), it depends on
  **`sqlx-core` + `sqlx-postgres` directly**, never the `sqlx` facade. Keep it that
  way — re-adding `sqlx = "0.8"` reintroduces the link conflict and breaks
  `make build-be`.
- **`serde_json`'s `preserve_order` is on (workspace-wide).** Needed so the column
  order `row_to_json` emits survives the parse. The feature is additive (Map →
  order-preserving); it only ever preserves more information.
- **Read-only is the *role's* job, not SQL parsing.** The crate never issues a
  write and refuses multi-statement input as belt-and-braces, but the real guarantee
  is the `SELECT`-only DB role (plus `default_transaction_read_only`, set on each
  connection). Don't rely on the single-statement check as a write guard.
- **No DataFusion in this path.** That's the whole point — TimescaleDB `time_bucket`,
  continuous aggregates, and chunk exclusion are only available because the external
  engine plans the query. Don't "unify" this with `rubix-query`.

## Known issues / fixes

Library built clean; no backend bug to record (there is no rubix backend behind it
yet). One **workspace-level build note** surfaced during this work, now resolved:
the `sqlx` facade pulled a second `libsqlite3-sys` that collided with `rusqlite`'s;
fixed by depending on `sqlx-core` + `sqlx-postgres` directly (see Gotchas). Verified:
`sqlx-sqlite` no longer appears in `Cargo.lock` and only `libsqlite3-sys 0.35` resolves.

> Separately: the workspace currently fails to load because a **different**,
> in-progress crate (`crates/rubix-rules/`) is listed in `members` but has no
> `src/lib.rs` yet — that is the rules session's to resolve, not this feature.
