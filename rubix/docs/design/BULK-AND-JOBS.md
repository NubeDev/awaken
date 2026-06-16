# BULK-AND-JOBS — bulk APIs and the long-running job spine

Forward design for **bulk operations** on rubix-server: bulk record CRUD, bulk
queries (incl. wide timeseries reads), and the **job + ticket** infrastructure
that lets any bulk op run either synchronously (quick) or as a long-running
background job streamed over WebSocket. Nothing here is shipped yet; this records
the architecture, the decisions taken at each fork, the build order, and what is
deliberately deferred. Where this note and [SCOPE.md](../SCOPE.md) disagree,
SCOPE.md wins.

## Problem

A bulk call has two failure-of-fit modes:

1. **Bulk create / update / delete** — N items, each gated and audited
   individually. The caller needs **per-item status** (item 3 of 200 failed
   validation; the rest committed), not one all-or-nothing verdict.
2. **Bulk read** — a single query that can be quick *or* can scan a large
   timeseries window and take seconds-to-minutes. The result may not fit one JSON
   body, and the caller may not want to hold a request open for it.

The client cannot reliably predict which mode it's in (a "small" reading window
can be large after a connector union). So **sync-vs-async is a server decision
behind a deadline, not a client flag** — see the Tier-1/Tier-2 split below.

## What already exists (build on, don't rebuild)

- **axum with `ws` enabled** and a working WebSocket bridge at `GET /ws/records`
  ([`ws/subscribe.rs`](../../crates/rubix-server/src/ws/subscribe.rs),
  [`ws/bridge.rs`](../../crates/rubix-server/src/ws/bridge.rs)). It is a
  *live-query subscription* (SurrealDB change feed → JSON frames), authorized once
  at subscribe time. It is **not** a request/response or job channel — but it is
  the plumbing a job channel rides on.
- **Two synchronous bulk precedents.**
  [`POST /query/batch`](../../crates/rubix-server/src/http/query/batch.rs) runs up
  to `MAX_BATCH = 50` keyed statements against one shared context with **per-item
  error isolation** (HTTP 200 unless the envelope is malformed; each item returns
  rows *or* an error). [`POST /readings`](../../crates/rubix-server/src/http/readings/append.rs)
  is a bulk timeseries append with one capability check up front. These are the
  templates for Tier 1.
- **A near-perfect model for a job ticket.** The `session_token` table
  ([`auth_token.rs`](../../crates/rubix-gate/src/auth_token.rs)) is already a
  server-side, **hashed-at-rest, expiring, namespace-scoped, revocable** opaque
  token store (`issue`/`resolve`/`revoke`). A job ticket is a specialization of
  exactly this.
- **The "authorize once, never re-check per message" precedent**
  ([`ingest/subscribe/authorize.rs`](../../crates/rubix-ingest/src/subscribe/authorize.rs)):
  one `check_capability` at open, then the stream rides on that decision. This is
  the correct model for authorizing a long job once and letting cheap status polls
  ride a ticket.
- **A chunked engine layer.** DataFusion already produces `RecordBatch`es and
  exposes `DataFrame::execute_stream`; SurrealDB reads are range scans. The data is
  *already* streamable — the HTTP boundary just flattens it.

### What is missing

- No job-submit / poll-by-id / status surface anywhere.
- No result **pagination or streaming** — the query path does
  `dataframe.collect()` → one JSON array via `batches_to_rows`
  ([`http/query/render.rs`](../../crates/rubix-server/src/http/query/render.rs)),
  no LIMIT / cursor / chunking. This is the single choke point Tier 2 evolves.
- **No GraphQL**, anywhere. (See "Decision — no GraphQL" below.)

## Architecture: two tiers, server-decided

### Tier 1 — synchronous bulk (the common case)

Extend the `query/batch` shape to record CRUD and to fast reads. An envelope of
**keyed items**, per-item status, HTTP 200 unless the *envelope* itself is
malformed. Mutations loop `apply()` per item through the gate, so each item keeps
its own capability check + audit row + correlation id. Bounded item count and a
**soft deadline** (target ~5–10s).

If the work completes within the deadline → return the completed result. If it
would exceed the deadline (or the caller hit a `/bulk/jobs` endpoint for
inherently-heavy work) → **promote to Tier 2** and return `202` with a job id +
ticket. The client handles both the same way: *either a result, or a job handle.*

### Tier 2 — job + ticket, streamed over WebSocket

1. `POST /bulk/jobs` → `Authenticated` extractor →
   **`check_capability` once** → spawn the work on `tokio::spawn` → mint a
   short-TTL **ticket** → return `202 { job_id, ticket, expires }`.
2. **Progress + results over the existing WS plane**, not new SSE. The client
   connects `GET /ws/jobs/{job_id}` and presents the ticket via the
   **`Sec-WebSocket-Protocol` subprotocol** (`rubix-job-ticket, <ticket>`), *not* a
   `?ticket=` query string. Rationale: the browser `WebSocket` API cannot set
   custom headers, so subprotocol is the only header-grade, browser-compatible
   channel; a query-string ticket lands in access logs and history. The server
   streams typed frames — `{item_key, status}` events for bulk-create (the "status
   per new item"), and **result `RecordBatch` chunks** for big queries. Frame auth
   is a cheap hash + expiry + `job_id` match; **no capability re-check** per frame.
3. `GET /bulk/jobs/{job_id}` is the **fallback poll** (same ticket, via
   `Authorization` header). It returns **status always**, and the terminal *result*
   **only for jobs whose result is small and fully buffered** — i.e. CRUD per-item
   status lists. For **streamed query jobs the poll is status-only**: their rows
   are never fully buffered (that would re-create the huge-JSON-body problem the
   design exists to avoid), so the rows are available *only* by consuming the WS
   stream. The poll tells such a client `result_transport: "stream"` so it knows to
   (re)connect the socket rather than wait for rows on the poll.

This gives one mental model across every bulk op and reuses the WS plane SCOPE
OQ2 already flags as the load-bearing bet — it does **not** add a second bus.

## The job spine (built first)

Per the long-term decision, the **spine is built before either use case**, so
bulk CRUD and streaming query are both thin layers over shared infra rather than
two divergent one-offs.

### Job registry — in-memory

Decision: **in-memory `Arc<RwLock<HashMap<JobId, JobState>>>`** held in
[`AppState`](../../crates/rubix-server/src/state.rs), not persisted. `JobState`
carries the namespace, the originating subject, a status enum
(`Running { done, total } | Completed | Failed { reason }`), a `CancellationToken`,
and the fan-out channel below.

**Fan-out — `tokio::sync::broadcast` + a bounded backlog ring, not `mpsc`.** There
can be more than one observer (a WS consumer *and* a reconnecting one, or a poll
alongside a socket), so the channel must be multi-consumer — `mpsc` is
single-consumer and is rejected. To serve a **late or reconnecting** subscriber,
the job keeps a bounded ring of its most recent frames: a new subscriber **replays
the ring, then tails the broadcast**. The ring is *not* a full-result buffer — for
streamed query jobs it holds only the last K chunks (reconnect window), so a
dropped socket can resume without re-running the query but the full result is never
materialized in memory. CRUD per-item statuses are small and bounded by item count,
so for those the ring holds the complete terminal list (and that is what poll
returns).

**Limits — this is a resource-exhaustion surface, so it is bounded explicitly.**
Max concurrent *running* jobs per principal and per namespace, plus a max queued
depth; submission over the cap returns `429` (not a queued job that never runs).
Each job has a hard wall-clock timeout → `Failed { reason: "timeout" }`. The
`CancellationToken` is tripped on explicit `DELETE /bulk/jobs/{id}`, on ticket
expiry, and on timeout. **A dropped WS connection does *not* cancel the job** — a
half-committed bulk mutation must run to completion regardless of who is watching;
the client reconnects or polls. (The one exception is a streamed query whose only
consumer has gone *and* whose backlog ring has filled — there the producer would
otherwise block forever, so it is cancelled on the job timeout.)

Rationale for in-memory: the edge deploy is a single embedded SurrealDB node; a
durable queue is unjustified complexity for the first cut. **Consequence, stated
plainly:** jobs do **not** survive a server restart — an in-flight job is lost and
the client must resubmit. After restart the registry is empty, so any ticket
resolves to "job unknown" (see below) — a safe-fail, not a security hole. Durable,
resumable jobs (a SurrealDB-backed `job` table + cleanup sweep) are a later layer
if multi-node or long-horizon jobs arrive — the registry trait is the seam.

A background sweeper evicts terminal jobs after a short retention (so a completed
result is pollable for a grace window, then dropped) to bound memory.

### Job ticket — copy `session_token`

A new `job_ticket` table modeled directly on `session_token`
([`auth_token.rs`](../../crates/rubix-gate/src/auth_token.rs)): `token_hash`
(sha256 of an opaque 256-bit value, raw returned once), `job_id`, `subject`,
`namespace`, `expires`. Functions `issue_job_ticket` / `resolve_job_ticket` /
`revoke_job_ticket` mirror the session-token trio.

Decision — **a parallel table, not reuse of `session_token`.** A session token
resolves to *full principal credentials*; a job ticket must resolve only to "may
observe this one job." Narrower scope, separate lifetime (minutes, not 24h).
Reusing `session_token` would over-grant.

**Lifetime — revoked on expiry or eviction, *not* on job completion.** The ticket
must outlive completion so the client can read terminal status/results during the
grace window (a ticket killed the instant a job finishes would make the result
unfetchable). The ticket TTL is therefore sized to cover *expected job duration +
grace window*; explicit `DELETE /bulk/jobs/{id}` revokes early. When the sweeper
evicts a terminal job it also revokes the ticket, so the two lifetimes stay aligned.

`resolve_job_ticket` validates, every call: hash match **and** unexpired **and**
`ticket.job_id == path job_id` **and** `ticket.namespace == job.namespace`
**and the job still exists in the in-memory registry**. The last check is what
makes restart safe — orphan ticket rows whose job is gone resolve to "job unknown"
(client resubmits). A periodic sweep deletes `job_ticket` rows where
`expires < now` so orphans don't accumulate after a restart. Tenancy is stamped on
the job and ticket rows from the **authenticated principal**, never from the request
body (mirrors readings/records).

**Bearer semantics, stated explicitly:** the ticket is a bearer token —
`resolve_job_ticket` does *not* re-check that the presenter is the original
`subject`, so any holder of a valid ticket may observe that one job. This is
intentional (it lets a client hand the ticket to a worker/tab that opens the
socket) and is bounded by the short TTL and single-job scope. The stored `subject`
is for audit/attribution, not an access gate. If per-presenter binding is ever
wanted, add the subject check here — it is a one-line tightening, not a redesign.

### WS job channel

A second WS route family alongside `/ws/records`:
`GET /ws/jobs/{job_id}`. Upgrade → resolve ticket → subscribe to that job's
broadcast → forward typed frames until terminal, then close. This reuses the
`bridge.rs` forward pattern; it does **not** reuse the live-query data plane (a job
channel forwards from the in-memory registry, not from a SurrealDB live query).

## Bulk record CRUD (layer 2)

`POST /records/bulk` — envelope `{ items: [{ key, op, body }] }`, `op` ∈
create/update/delete. Tier-1 path loops `apply()` per item, collecting
`{ key, status, id?, error? }`; per-item isolation exactly like `query/batch`.
When promoted to Tier 2, the same loop runs in the spawned job and emits each
`{ key, status }` as a WS frame as it commits. The gate path is unchanged — bulk
is purely a server-side fan-out over the existing single-item `apply()`.

**Promotion contract for mutations.** A bulk mutation can promote *after some items
have already committed* (their side effects are real and irreversible). To keep the
client's picture complete: the `202` body carries the statuses of every item that
finished **before** promotion, and the WS stream carries the rest — both keyed by
the item `key`, so the union of `202` + frames is the full result with no gap and no
double-report.

**Idempotency, honestly scoped.** The item `key` is a **correlation** key (it ties a
status frame back to a submitted item); it is **not** a cross-request idempotency
key in v1. Re-submitting a promoted job after a disconnect therefore risks
double-create. Guidance for v1: use `update`/upsert `op`s (idempotent by id) where
re-runs are possible, and treat `create` as at-least-once. A real idempotency store
(dedup on a client-supplied key across submissions) is **deferred** — called out in
Open Questions, not built here.

Likely a new closed-enum `Capability` variant (e.g. `bulk-submit`) in
[`capability/kind.rs`](../../crates/rubix-gate/src/capability/kind.rs) to gate job
submission distinctly from the per-item caps. **`bulk-submit` authorizes only the
act of opening a bulk job — it never authorizes the underlying mutations or reads.**
Each item still flows through `apply()` and is checked against its own capability +
row-level perms, so a principal with `bulk-submit` but no `record:write` gets a job
whose every item fails authorization. The bulk cap gates the *resource* (spawning a
job), the per-item caps gate the *data*. **Note:** `ALL` and the round-trip registry
test asserting the count (currently `== 11`) must change together.

## Streaming query (layer 3) — the timeseries driver

This is the part the client was least sure about, and it is why the spine exists.
A wide reading scan can't be one JSON blob. The fix is to stop flattening at the
HTTP boundary: for a Tier-2 query job, replace `batches_to_rows` + `collect()`
([`http/query/render.rs`](../../crates/rubix-server/src/http/query/render.rs))
with `DataFrame::execute_stream`, and push each `RecordBatch` as a WS result frame
(Arrow-JSON per chunk, with a final `{ done: true }` frame). The engine is already
chunked; only the boundary changes.

Tier 1 still serves quick queries inline (unchanged `query/run` path); the deadline
promotes a slow one to a streamed job transparently.

## Decision — no GraphQL

SurrealDB supporting GraphQL does **not** help here, and we do **not** adopt it.
Reads do not go through SurrealDB's query surface directly — they go through
**DataFusion** (cross-source unification, vectorized rollups,
[`rubix-query`](../../crates/rubix-query)) and gate-scoped sessions. A GraphQL layer
would sit *beside* that, requiring a second auth/capability integration, a second
schema, and a second subscription/streaming story — for no gain over the REST + WS
already in the tree. The genuinely hard parts here (long-running jobs, per-item
status, chunked timeseries) are exactly what GraphQL is weakest at. Revisit only if
an external consumer specifically demands GraphQL; it is not on the path.

## Build order

1. **Spine** — `job_ticket` table + `issue/resolve/revoke`; in-memory job registry
   in `AppState`; `GET /ws/jobs/{id}` + `GET /bulk/jobs/{id}` poll; eviction
   sweeper. Tested with a trivial synthetic job.
2. **Bulk record CRUD** — `POST /records/bulk`, Tier-1 sync with per-item status;
   deadline promotion to a Tier-2 job emitting per-item WS frames; `bulk-submit`
   capability.
3. **Streaming query** — `DataFrame::execute_stream` render path; deadline-based
   promotion of a slow query to a streamed job.

## Open questions

1. **Deadline, item caps, and concurrency limits.** Where exactly does Tier 1 stop
   trying and promote, and what are the per-principal / per-namespace running and
   queued caps that return `429`? All need numbers per op class; start conservative
   (~10s deadline, ~500 items, small concurrency) and tune against real timeseries
   scans.
2. **Backlog ring depth (K) for streamed queries.** The fan-out decision (broadcast
   + bounded ring) is made; the remaining tunable is how many recent chunks the ring
   holds — the reconnect window. Too small and a brief disconnect loses resumability;
   too large and it drifts toward the full-result buffering this design rejects.
3. **Cross-request idempotency.** v1 uses the item `key` only for correlation, so a
   resubmitted `create` job can double-create. A real dedup store (client-supplied
   idempotency key, persisted) is deferred — decide if/when at-least-once `create` is
   unacceptable for a consumer.
4. **Promotion visibility.** A Tier-1 call that promotes returns `202` mid-flight,
   carrying already-committed item statuses — the client must accept that a "sync"
   endpoint can hand back a job handle. Confirm the client contract tolerates this
   rather than splitting into separate sync/async endpoints.

## Test surface (planned)

- `rubix-gate`: `job_ticket` issue/resolve/revoke round-trip, expiry, namespace
  isolation, `job_id` mismatch rejection, **resolve-fails-when-job-absent** (restart
  safety), and the expired-row sweep; `Capability` registry covers `bulk-submit`.
- `rubix-server`: job registry lifecycle + eviction; **terminal job stays pollable
  through the grace window, then 404s**; ticket survives completion and is revoked
  only on eviction/expiry/`DELETE`; concurrency cap returns `429`; job runs to
  completion after the WS drops; cancellation on `DELETE` and on timeout;
  `/ws/jobs/{id}` accepts the ticket via subprotocol, replays the backlog ring to a
  late subscriber, and rejects a bad/expired ticket; `POST /records/bulk` per-item
  isolation (one item fails, rest commit) and **`bulk-submit` without the per-item
  cap yields all-items-forbidden**; deadline promotion returns `202` with
  already-committed statuses + ticket; streaming-query poll is status-only while the
  WS carries chunked frames + terminal `done`.
