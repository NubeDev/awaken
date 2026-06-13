# Datasources — External Read-Only Data

Scope for reading external databases (e.g. a customer's TimescaleDB historian)
into rubix dashboards, sparks, and the AI query tool, without copying the data
into the rubix store and without routing it through the DataFusion query layer.

## Problem

A site already has its history in an external database — concretely a TimescaleDB
hypertable holding hundreds of GB. Requirements:

1. The data stays where it is. Rubix does not copy or sync it.
2. Reads must use the external engine's full capability — TimescaleDB
   `time_bucket`, continuous aggregates, chunk exclusion, compression — because
   those are what make a dashboard page load fast over that volume.
3. Dashboards, sparks, and the AI layer must be able to read it, using the same
   render/finding paths they already use for rubix-owned data.

## Why not DataFusion federation

`rubix-query` federates rubix's own tables through DataFusion (and could federate
external SQL via `datafusion-table-providers`). That path is rejected for this
use case:

- DataFusion plans the query, not the external engine. TimescaleDB-specific
  constructs (`time_bucket`, continuous aggregates, hypertable chunk exclusion,
  `last`/`first`, `SkipScan`) are not expressible or are lost, so the external
  optimizations that make large-volume reads fast do not apply.
- The generic provider pattern materializes a bounded `SELECT *` slice and filters
  above it in-process — acceptable for a small exception-path join, wrong for a
  large historian where the point is to push aggregation down to the source.

Federation stays the right tool for joining rubix's own relational tables. It is
not the tool for a large external historian.

## Model

A **datasource** is a declared, read-only connection to an external SQL database
that runs native SQL and returns rows. DataFusion is not in the path; the external
engine plans and executes the query.

```
dashboard widget ─┐                    ┌─ registry: owns creds + pool per id ─┐
spark node        ├─► datasource ──────┤                                      │
AI (named only)   ─┘    executor       └─► [native SQL] ─► external DB         │
                           │                                    │             │
                     caps + read-only role               rows back            │
                           └──────────── columns + rows ◄───────┘─────────────┘
```

The returned shape (`columns + rows`) is identical to what `rubix-query` returns,
so every consumer renders or folds it the same way it already does.

### Query authoring tiers

The native SQL is not a free-text field shared by all consumers. Authoring trust
differs by consumer, so the binding does too:

- **Operator-authored SQL** (dashboard widgets, spark nodes) — an operator writes
  the native SQL in the widget/board definition. Trusted-but-capped.
- **Named queries** (the AI tool) — an operator pre-registers named, parameterized
  queries on the datasource. The AI may *invoke* a named query with bound
  parameters; it may not author raw SQL. See [AI](#consumers).

Both tiers run through the same executor and the same caps. The difference is who
supplies the SQL text.

### Parameterization

Bindings never textually splice values into SQL. A binding declares the native SQL
with positional/named placeholders and a typed parameter list; the executor passes
parameter values to the database driver as bound parameters (`$1`-style), separate
from the SQL text. A dashboard's time-range picker and site filter become declared
parameters, not string interpolation. Textual interpolation into native SQL is
forbidden — it would reintroduce injection inside the "trusted" SQL path.

### Two read paths, deliberately separate

| Path | Engine | Reads | SQL dialect |
| --- | --- | --- | --- |
| `rubix-query` | DataFusion over SQLite/Postgres | rubix-owned `sites`/`equips`/`points`/`his`/`sparks` | DataFusion SQL |
| datasource | direct client, native | external databases (the historian) | the external engine's native SQL |

These do not merge into one SQL surface. Merging them (via federation) is exactly
what discards the external optimizations. They remain two paths that both feed the
same consumers.

## Boundaries

A datasource is distinct from a **protocol driver** (`drivers.json`). They flow in
opposite directions and must not share a registry or config:

- **Protocol driver** — a supervised child process that *writes* live point data
  *into* the rubix store over zenoh (BACnet, Modbus, sim).
- **Datasource** — a read-only connection rubix *reads out of* at query time.

Declared in their own manifest (`datasources.json`), consumed only by the
datasource executor. Neither knows about the other.

## Consumers

- **Dashboards** — a widget data-binding gains one new kind,
  `{ datasource, sql }`, alongside the existing `{ query }` binding. Same widget
  registry, same render path, same SSE seam. The widget carries native SQL.
- **Sparks** — a new spark node (sibling to the existing history-query node) runs
  a datasource query, and the rest of the board computes on the rows and emits a
  finding through the existing finding emitter. The spark engine, the board
  runner, and the finding path are unchanged. This is the SkySpark "rule reads a
  SQL grid and folds it into a finding" pattern.
- **AI** — a read-only tool that **invokes operator-registered named queries**
  with bound parameters, parallel to the existing `query` tool and under the same
  gating. The AI does not author raw SQL against a datasource. Rationale: the AI
  generates calls at runtime from model context, which is a prompt-injection
  surface; granting it raw SQL against a customer's production historian would
  contradict the operator-authored trust model. Named-query invocation keeps the
  SQL operator-authored while letting the AI parameterize it. A datasource opts in
  to AI access explicitly, and may carry a stricter role / lower caps for the AI
  tier.

All three reach the external DB through one executor over one pool per datasource.
There is no second client stack and no per-consumer connection logic.

## Safety

Raw native SQL authored in a widget/spark/tool against a large external
production database is the real surface. Required, not optional:

- **Read-only role.** The datasource connects as a user with `SELECT`-only
  grants. Write prevention is enforced by the database role, not by parsing SQL.
  The role is the primary mechanism but does not cover everything: a `SELECT`-only
  user can still call functions it has `EXECUTE` on (default `PUBLIC` grants),
  `pg_sleep`, take `ACCESS SHARE` locks, and run multi-statement strings if the
  client sends them. Recommended role setup therefore also revokes default
  function `EXECUTE` and sets `default_transaction_read_only`. See single-statement
  below.
- **One statement per call.** The executor sends exactly one statement per call
  and rejects multi-statement input, so a `SELECT`-only role cannot be bypassed by
  a trailing statement in the same string.
- **Wall-clock cap.** Enforced via the engine's native mechanism where available
  (e.g. Postgres/Timescale `statement_timeout`) and client-side cancellation
  otherwise, so a runaway aggregate is killed rather than left to hang a node.
- **Row/byte caps** on the returned result, so a single read cannot pull an
  unbounded result into a node or a browser. Breach truncates and is reported on
  the dashboard path; on the spark path a breach is an error, not a finding (see
  [Refresh cost](#refresh-cost) and below).
- **Per-datasource concurrency cap.** Each datasource has one pool with a small
  max size, so N widgets refreshing at once cannot open unbounded connections
  against the external database. The wall-clock cap bounds one query;
  the pool bounds concurrent queries.
- **Authoring trust.** v1 assumes operator-authored SQL (widgets, spark nodes) and
  operator-registered named queries (AI). End-user-supplied SQL is out of scope
  and would require separate, tighter handling.
- **Credentials** are owned by the datasource registry, keyed by datasource id.
  The registry resolves connection material (decrypted upstream) once and holds
  the pool; callers pass only the datasource id and never touch decrypted secrets.
  The registry is the only component that ever sees the password and never logs
  it. This keeps secret handling in one place rather than across all three
  consumers.

### Schema discovery

Both a human authoring a widget and the AI invoking a named query need to know
what tables and columns a datasource exposes. The registry provides a
**describe-datasource** capability — introspecting `information_schema` under the
read-only role, or returning an operator-declared schema blob from
`datasources.json`. Without it the authoring and AI surfaces are unusable in
practice.

## Tenancy

Datasource row data has no `{org}/{site}` columns, so the row-level tenant filter
that `rubix-query` applies to rubix-owned tables does not apply. Authorization is
per-datasource at the registry level: which tenants may reference which
datasource. An unscoped/edge-no-auth deployment is global by design, consistent
with `rubix-query`.

## Refresh cost

Reads run on the external engine on each call, so refresh cadence is a cost
decision the consumer owns:

- A live-updating dashboard widget that re-runs a heavy aggregate on a short
  interval, or a scheduled spark that fans out per site, can place real load on
  the external database — heavier than a single page load.
- Mitigation is in the authored SQL (read continuous aggregates that are cheap to
  re-read rather than scanning raw history each run), in cadence chosen against
  query cost, and in the per-datasource concurrency cap. The wall-clock cap is the
  backstop, not the strategy.

**Truncation on the spark path.** A dashboard rendering a truncated result is
acceptable — the user sees a capped view. A spark folding truncated rows into a
finding can silently reach a wrong conclusion even though the breach is reported.
So a caps breach on the spark path is an **error** that fails the node, not a
finding emitted from partial data.

## Non-goals

- No copy/sync of external data into the rubix store. The data stays in the
  external database.
- No write path. Datasources are read-only; sparks do not write to them.
- No DataFusion federation of the external historian.
- No merging of the datasource path and the `rubix-query` SQL surface.
- No non-SQL stores (e.g. document databases) in this scope; the model assumes a
  SQL engine whose native dialect rubix passes through.
- No end-user-authored SQL in v1.

## Relationship to existing components

- `rubix-query` — unchanged. The datasource path lives separately, with its own
  connection pool, and does not import DataFusion or the canonical tables.
- `rubix-flow` — gains one read-only actor; the actor model, board runner, and
  finding emitter are unchanged.
- `drivers.json` / protocol drivers — unrelated and unchanged; datasources are a
  separate manifest and a separate direction of data flow.
