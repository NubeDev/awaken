# Flow runtime redesign — scope & gaps

Working notes for moving the flow board runtime from a rebuild-per-tick model to a
persistent, Niagara/Sedona-style scan engine with a pushed live-value bus. Captures
(1) how the system works today, (2) the design gaps in the flow↔server seam that the
redesign must close first, and (3) the staged scope.

Source crates: [`rubix-flow`](../../../crates/rubix-flow), the engine wrapper, and
[`rubix-server`](../../../crates/rubix-server) `scheduler/` + `flow/` + `api/boards/`,
the host. UI under [`rubix/ui`](../../../ui) `src/features/flows` + `src/api`.

---

## 1. How it works today

### Engine
The execution engine is **reflow** (`reflow_network` / `reflow_actor` 0.2, external
crates). `rubix-flow` wraps it: a `BoardGraph` (JSON) is loaded into a reflow `Network`
of actors, one per node, wired by the graph's connections. Nodes depend on the host only
through the `PointAccess` trait — `rubix-flow` carries no axum/sqlite/zenoh.

### Execution model — rebuild per tick
`BoardGraph::run` ([`board/run.rs`](../../../crates/rubix-flow/src/board/run.rs)) is a
**single-shot** evaluation:

1. `load(access)` builds a brand-new `Network` with fresh actor instances.
2. `network.start()`.
3. Tick each source node (no inbound connection) once on its first inport.
4. Drain outports every 50ms until a poll yields nothing new (settled) or the 120s budget.
5. `network.shutdown()` — the whole network is destroyed.

The scheduler ([`scheduler/interval.rs`](../../../crates/rubix-server/src/scheduler/interval.rs))
calls this once per interval. So an "every 1s" board **constructs and demolishes the entire
actor network once per second.**

Consequence: actor state cannot survive between ticks. The `trigger` node therefore stashes
its clock/count/level in a process-global `static HashMap`
([`node/trigger/trigger_state.rs`](../../../crates/rubix-flow/src/node/trigger/trigger_state.rs)) —
a workaround for an engine with no persistent state. The repeated `[INPORT CLOSED]` logs are
this teardown/rebuild firing every tick (printed by reflow's `ActorProcess::run` when an inport
closes); they persist until the persistent engine lands.

> **Resolved (load):** reflow's default `NetworkConfig` enables a tracing client that dials a
> `ws://localhost:8080` server on every `Network::start`; no such server runs here, so the
> default produced a reconnect storm — one dial per board build. `BoardGraph::load` now builds
> the network with tracing disabled, so that storm is gone independent of the rebuild model.

### Run modes
Server models three triggers
([`scheduler/trigger.rs`](../../../crates/rubix-server/src/scheduler/trigger.rs)): `Manual`
(HTTP `/run` only), `Interval{seconds}` (min 1s), `Subscription{key}` (fires on a zenoh
keyexpr sample). The UI exposes two ("Continuous — fixed interval" vs "On demand") plus an
Enabled toggle — over-exposed, and redundant with the `trigger` node's own period.

### Live values — REST polling, latest-only
Node outputs are recorded into an in-memory latest-only cache
([`scheduler/outputs.rs`](../../../crates/rubix-server/src/scheduler/outputs.rs)), replaced
wholesale each run. The UI polls `GET /boards/{slug}/outputs` every 5s
([`ui/src/api/hooks.ts`](../../../ui/src/api/hooks.ts)). There is **no SSE or WebSocket
anywhere** in the server or UI. The Points page and Dashboards also poll on the same 5s timer.

### Why live values look buggy/empty
- 5s poll vs variable run cadence → stale (60s board) or missed runs (1s board).
- Manual/disabled boards never populate the cache → endpoint returns `[]` → blank.
- Editing suppresses values: while `dirty`, `liveByNode` returns empty by design
  ([`ui/src/features/flows/index.tsx`](../../../ui/src/features/flows/index.tsx)).
- Test Run sets `lastValues = byNode.get(id)`, which is `undefined` for nodes not in that
  run → wipes them.
- Latest-only cache: one empty run blanks the panel (no last-known-good).

---

## 2. Design gaps in the flow↔server seam

The seam's **shape is good** and worth keeping: a host-implemented `PointAccess` trait
([`port.rs`](../../../crates/rubix-flow/src/port.rs)) that keeps `rubix-flow` transport-free,
with `bail!`-by-default methods that **fail closed** for unsupported capabilities. The gaps are
that it is **synchronous, pull-only, and untyped at the value/error level** — all of which the
persistent scan engine amplifies and so must be closed before building the scan loop on top.

### G1 — Synchronous seam over async I/O — **closed**
Was: every `read_point` / `write_point` / `query_his` / `query_datasource` /
`request_agent_blocking` was sync, and the host bridged to async with `block_in_place` +
`block_on` ([`flow/access.rs`](../../../crates/rubix-server/src/flow/access.rs) — datasource and
awaited-agent paths), parking a Tokio worker for the whole round-trip, including the **LLM call**
for an awaited `agent_call`.

`PointAccess` is now an `#[async_trait]`; the datasource and agent paths `.await` directly with no
`block_in_place` bridge. `request_agent_blocking` is renamed `request_agent_awaited`. The remaining
SQLite point/history reads still run inline (sync within the async fn) — the same blocking profile
as today, no worse — and should move to `spawn_blocking` (or a dedicated blocking pool) **when the
persistent scan loop lands**, since that loop reads on a cadence rather than once per short-lived run.

### G1a — Cancellation & timeouts on the async seam (open)
Async removes worker-parking but not unboundedness: an awaited `agent_call` or a slow datasource can
still hang forever. The seam needs a deadline/cancellation discipline — a per-call `tokio::time::timeout`
at the engine boundary, and a cancellation token the scan loop and `watch` streams honor on
engine shutdown — so one stuck call cannot wedge a scan or leak a task. The single-shot `run()` has a
120s settle budget today; the persistent engine has no equivalent yet.

### G2 — Pull-only value model, no subscribe/watch (open)
The seam can only `read_point(one_keyexpr)`. The only event-driven path (the zenoh
`Subscription` trigger) lives in the scheduler, *outside* the seam. A persistent engine would
therefore poll every referenced point every scan — pushing the polling problem one layer down —
and the live-value bus would have nothing push-based to build on. The seam needs a
`watch(prefix) -> Stream` primitive so both the engine and the bus are event-driven.

> **Reconcile with the existing Subscription trigger.** `watch(prefix)` and the scheduler's
> `Subscription { key }` loop ([`scheduler/subscribe.rs`](../../../crates/rubix-server/src/scheduler/subscribe.rs))
> both declare a zenoh subscriber on a keyexpr. If a board both has a `Subscription` trigger and a
> node that `watch`es the same prefix, that is **two** subscribers and double-fires. `watch` must be
> the single subscription substrate: the `Subscription` trigger becomes a `watch` consumer, not a
> parallel path. And `watch(prefix)` must be **tenant-scoped** (org/site), or a board could subscribe
> to another tenant's points — the same authz boundary `ScopedPointAccess` already enforces on reads.

### G3 — Link values carry no quality/status/units
Outputs are a lossy `serde_json::Value::from(msg)`
([`board/run.rs`](../../../crates/rubix-flow/src/board/run.rs)). Niagara/Sedona links carry a
status flag (`ok / stale / fault / null / overridden`) on every value; here the only status
signal is a separate `error` outport. The retained-link-value model needs a `{value, quality, ts}`
shape, or a stale retained value is indistinguishable from a fresh one — turning "sometimes empty"
into "sometimes silently wrong."

### Lower-priority (address as those paths are touched)
- **G4 — `anyhow` across a stable seam — closed.** Replaced with a typed `FlowAccessError`
  (`Unsupported` / `Denied` / `Store`) ([`error.rs`](../../../crates/rubix-flow/src/error.rs)), landed
  alongside the G1 async reshape. `NotFound` is deferred until the store layer types its own lookup
  errors — today a missing keyexpr is `Store` and a point with no current value is `Ok(None)`.
- **G5 — `write_point` is fire-and-forget**: no writer provenance (board/run id), no slot
  lease/expiry, no device-accepted confirmation. A persistent engine re-commanding every scan
  wants idempotent lease semantics.
- **G6 — `PointAccess` is a god-trait** (points + sparks + agent + datasource + rules); capability
  is discovered at call-time via `bail!` rather than the type system. Could split into focused
  traits. Cleanliness, not a blocker.

---

## 2b. Keystone spike + review findings (verified against the crate)

The redesign rested on one unverified assumption. It is now spiked against `reflow_network` 0.2.1
itself (source read + an executable test,
[`tests/persistent_network_spike.rs`](../../../crates/rubix-flow/tests/persistent_network_spike.rs)).

**Keystone — confirmed.** A started `Network` *does* support a persistent model with no per-tick
rebuild:
- `Network::start` spawns one long-lived Tokio task per node (`ActorProcess::run`), a loop that
  reads the actor's inport and runs its behavior on each message until the inport closes. It does
  **not** exit after one message. `shutdown()` aborts those tasks.
- `send_to_actor` can be called repeatedly to re-tick a source; `read_actor_output` drains an
  outport non-blocking. Both work on a live network — no `shutdown()` between scans.
- Each actor's `state: Arc<Mutex<dyn ActorState>>` is created once per process and persists across
  loop iterations — so **Stage D's actor-held state is feasible** (the `trigger` state can leave the
  process-global map). No thin persistent-runtime layer over reflow is needed for the core loop.

**Finding 1 — observe link values via the event stream, not `read_actor_output`.** On `start`,
every source actor's outport is consumed by a fan-out forwarder that delivers to downstream inports.
So `read_actor_output` on a *connected* node races the forwarder and reliably yields **only terminal
(no-outbound) nodes** — the existing single-shot `run()` already relies on this (see the comments in
[`tests/board.rs`](../../../crates/rubix-flow/tests/board.rs)). But every wired link emits a
`NetworkEvent::MessageSent { from_actor, from_port, message }` on `Network::get_event_receiver()`.
**That event stream is the complete, race-free tap** for the live-value bus — terminal-node outputs
come from `read_actor_output`, all interior links from `MessageSent`. The spike test asserts both.
The doc's "scan = tick sources + drain outputs" is therefore incomplete; the engine must subscribe
to the event stream.

**Finding 2 — the event channel leaks unless drained.** `network_event_emitter` is an *unbounded*
flume channel the `Network` holds for its lifetime; every `send_to_actor` and every link pushes an
event. Single-shot runs drop the `Network` each tick, so it is GC'd. A **persistent** engine that
never drains `get_event_receiver()` grows without bound. The engine must continuously drain it
(which is convenient — it is also the value source from Finding 1). Only one consumer may drain it
(flume MPMC splits events across receivers).

**Finding 3 — `shutdown()` alone leaks the forwarder tasks.** The fan-out forwarders and per-connector
delivery tasks are bare `tokio::spawn`s, not tracked in `Network::processes`; `shutdown()` aborts only
the actor processes. The forwarders exit only when their channels disconnect, which happens when the
`Network` (holding the outport senders) is **dropped**. So engine teardown/republish must **drop** the
`Network`, not just call `shutdown()`, or every republish leaks tasks.

**Finding 4 — scan overrun / back-pressure.** Even async, if a scan's I/O outlasts the scan period,
ticks pile up. The interval loop today uses `MissedTickBehavior::Skip` — keep that model: at most one
in-flight scan per board, coalesce/skip overruns, and bound each scan with a timeout (G1a).

**Finding 5 — re-commanding writes every scan.** A persistent engine that re-runs `write_point` every
scan pushes a priority-array command (→ history, audit, bus publish) on every cycle even when nothing
changed. This makes G5 a near-term correctness/cost issue, not a deferral: **coalesce unchanged writes**
(command only when value or priority actually changes) before the scan loop ships.

**Finding 6 — engine lifecycle on republish/disable.** Republish must stop the old scan loop, drop the
old `Network` (Finding 3), close the old event drain + broadcast channel, then build the new one — and
SSE subscribers should be reseeded from the snapshot across the swap. Disable must **tear the engine
down**, not merely skip ticks as the interval loop does today.

**Finding 7 — multi-tenant authz on SSE.** `GET /boards/{slug}/outputs/stream` must enforce the same
tenant/capability gate as the REST `/outputs` endpoint; board link values can carry a tenant's point
values, so the stream is tenant-scoped data, not public.

**Finding 8 — testing a time-based persistent engine.** Inject the scan clock/tick source (the
`trigger` node already injects `now: Instant` — extend that pattern) and use `tokio::time::pause`/
`advance` so scan-loop tests are deterministic; assert via the event-stream tap, not sleeps.

---

## 3. Proposed scope (staged)

Ordered so nothing is thrown away: the persistent network and the seam redesign are the
foundation the rest builds on. No board-level rebuilds after the first stage.

### Stage A — Seam redesign + persistent network (foundation)
- **Done:** `PointAccess` is `async` with a typed `FlowAccessError` (closes G1, G4); the
  `block_in_place` bridges are gone. The tracing dial is disabled at `load` (no more
  `ws://localhost:8080` storm). The keystone is spiked (§2b).
- **Next — `watch(prefix) -> Stream` (G2):** add it to the seam as a `BoxStream` of
  `{keyexpr, value, quality, ts}`, tenant-scoped, with a fail-closed default; fold the scheduler's
  `Subscription` trigger onto it so there is one subscription substrate (§2b / G2).
- **Next — `BoardEngine`** owning one started reflow `Network` for its lifetime:
  `spawn` / `scan` / `current_values` / `shutdown`. Replace per-tick build→run→shutdown in
  `scheduler/interval.rs` with build-once + a scan loop; `Interval{seconds}` is reinterpreted as the
  **scan rate**. Design constraints from the spike: observe link values from the
  `NetworkEvent::MessageSent` stream + terminal `read_actor_output` (Finding 1); continuously drain
  the event receiver (Finding 2); on scan, bound each cycle with a timeout and run at most one
  in-flight scan, skipping overruns (Finding 4, G1a); coalesce unchanged `write_point` commands
  (Finding 5).
- Keep the one-shot `run()` only for Test Run and Manual `/run`.
- Lifecycle: scheduler `register`/`unregister` spawn/kill an engine; republish/disable **drops** the
  old `Network` (not just `shutdown()`) and rebuilds, reseeding SSE subscribers from the snapshot
  (Findings 3, 6).
- Move the SQLite point/history reads to `spawn_blocking` here, where the scan loop reads on a
  cadence (G1).

### Stage B — Live value bus + SSE
- Per-board broadcast channel fed by the engine's drain of `NetworkEvent::MessageSent` + terminal
  outputs (§2b Finding 1); publishes `{value, quality, ts}` deltas (closes G3 at the link level).
  New `GET /boards/{slug}/outputs/stream` (axum SSE) emits the current snapshot on connect, then
  deltas. Keep `GET /outputs` REST for the snapshot. **Gate the stream with the same tenant/capability
  check as `/outputs`** (Finding 7).
- UI: `useBoardOutputsStream` via `EventSource`; seed from snapshot, retain last-known-good per
  port, drop the 5s poll. Stop blanking on `dirty` / Test-Run / missing-node; show freshness age.

### Stage C — Simplify run modes to enable/disable
- Strip the "Continuous / On demand" dropdown; leave Enabled + an optional advanced **Scan rate**.
  Intra-graph rate stays on the `trigger` node. `Manual` stays server-side for the Test Run button.

### Stage D — Persistent component state (true scan model)
- Migrate `trigger` (and any stateful node) off the process-global `static HashMap` to
  actor-held state, now that actors live across scans. Removes the registry hack and the
  restart-coupling of `boot`.
- Retained link values become the source of truth — each link holds a current value
  continuously, making the live bus complete by construction.

### Stage E — Unify the live bus across the app
- Points page and Dashboards subscribe to the same SSE/zenoh-backed stream instead of 5s
  polling — one real-time substrate for points, board links, and widgets.

### Deferred
- G5 (write provenance/lease) — revisit when wiring the persistent writer.
- G6 (split the god-trait) — opportunistic.

---

## Dependencies
```
Stage A (async seam + persistent net) ─┬─> Stage B (SSE) ──> Stage C (UX)
                                        └─> Stage D (component state) ──> Stage E (unified bus)
```
Stage A is the keystone; its one real unknown (reflow's long-lived-network behaviour) is now
spiked and confirmed (§2b), and the seam half (async + typed error) has landed. The remaining
Stage A work is the `watch` primitive + the `BoardEngine`, built to the §2b findings. Stages B–C
deliver the visible win (real-time, no flicker, simple UX); Stages D–E complete the
Niagara/Sedona model.
