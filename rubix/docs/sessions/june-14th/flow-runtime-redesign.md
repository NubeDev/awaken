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

### G2 — Pull-only value model, no subscribe/watch — **closed**
The seam now has `watch(prefix) -> BoxStream<WatchSample>`
([`port.rs`](../../../crates/rubix-flow/src/port.rs)), the push-capable counterpart to
`read_point`, fail-closed by default and implemented on `StorePointAccess` over the zenoh bus (a
forwarder task owns the subscriber; dropping the stream undeclares it). The scheduler's
`Subscription` trigger ([`scheduler/subscribe.rs`](../../../crates/rubix-server/src/scheduler/subscribe.rs))
now consumes `watch` instead of declaring its own subscriber, so there is **one** subscription
substrate — a future `watch`-consuming node and the trigger can share a key without double-firing.

> **Still to do:** tenant-scope `watch(prefix)` in `ScopedPointAccess` (it currently delegates;
> subscription keys are operator-authored, like the board, so this is hardening not a hole), and have
> the `BoardEngine` consume `watch` so a scan is driven by a change rather than the fixed cadence.

### G3 — Link values carry no quality/status/units — **closed (quality)**
Every retained link value now carries a `Quality` (`ok`/`fault`/`null`) derived at capture — an
`error`-port value is `fault`, a JSON null is `null`, else `ok`
([`board/run.rs`](../../../crates/rubix-flow/src/board/run.rs)). `NodeOutput` and the server
`PortOutput` (REST + SSE) carry it, so a stored value is self-describing. Units negotiation on link
values is still future (the `rubix-prefs` unit layer is a separate seam).

### G3 (original note) — Link values carry no quality/status/units
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
- **Done — async seam:** `PointAccess` is `async` with a typed `FlowAccessError` (closes G1, G4);
  the `block_in_place` bridges are gone. The tracing dial is disabled at `load` (no more
  `ws://localhost:8080` storm). The keystone is spiked (§2b).
- **Done — `BoardEngine`** ([`board/engine.rs`](../../../crates/rubix-flow/src/board/engine.rs)):
  owns one started `Network` for the board's lifetime; `spawn_engine` / `scan` / `current_values`.
  `scan` re-ticks the sources and folds link values into a retained per-`(node,port)` snapshot —
  interior links from the `NetworkEvent::MessageSent` tap, terminals from `read_actor_output`
  (Finding 1) — draining the event channel each scan (Finding 2). Dropping the engine tears the
  network down (Findings 3, 6).
- **Done — interval wiring:** `scheduler/interval.rs` builds the engine once and *scans* it every
  `seconds` (the scan rate), rebuilding on a `version` bump (republish) and dropping it on disable.
  The one-shot `run()` stays for subscription, on-demand, and Test Run.
- **Done — `watch(prefix)` (G2):** added to the seam as a `BoxStream<WatchSample>`, fail-closed by
  default, implemented over zenoh; the `Subscription` trigger now consumes it (one substrate).
- **Done — scan-loop robustness:** unchanged `write_point` commands coalesce via actor state
  (Finding 5); SQLite reads run on `spawn_blocking` and the agent/datasource calls are timeout-bounded
  (Finding 4, G1, G1a). At-most-one-in-flight scan holds structurally (the interval loop awaits each
  scan; `MissedTickBehavior::Skip` drops pile-ups).
- **Still to do:** drive a scan from a `watch` change (event-driven scan) rather than only the fixed
  cadence; tenant-scope `watch` in `ScopedPointAccess`.

### Stage B — Live value bus + SSE
- **Done — server:** `BoardOutputs` holds a per-board `tokio::sync::broadcast`; every `record`
  (scheduled scan, subscription sample, on-demand run) pushes the snapshot, so the stream and the
  REST snapshot share one feed. `clear` pushes an empty frame so a disabled/deleted board blanks
  subscribers. `GET /api/v1/boards/{slug}/outputs/stream` (axum SSE) emits the snapshot on connect
  then each subsequent snapshot, under the same tenant authorization as `/outputs` (Finding 7).
- **Done — UI hook + editor wiring:** `useBoardOutputsStream` reads the stream over `fetch` (native
  `EventSource` can't send the bearer header), retains last-known-good per `(node, port)` so a
  momentary empty run no longer blanks the canvas, and reconnects with backoff. The flow editor is
  wired onto it, replacing the 5s poll for live boards.
- **Next:** snapshots are full pictures, not yet field-level deltas (the SSE frame already carries
  `quality`, so the editor can render the flag and a freshness age). Note: the editor's
  *dirty-suppression* of live values is intentional — it lets a Test Run preview unsaved edits
  without the live stream overwriting them — so it stays.

### Stage C — Simplify run modes to enable/disable
- Strip the "Continuous / On demand" dropdown; leave Enabled + an optional advanced **Scan rate**.
  Intra-graph rate stays on the `trigger` node. `Manual` stays server-side for the Test Run button.

### Stage D — Persistent component state (the node-state system)
- **Done — node-state contract.** A stateful node no longer hand-rolls persistence; it declares an
  explicit [`StatePolicy`](../../../crates/rubix-flow/src/state.rs) per call against a board-scoped
  `NodeState` store. There is **no default** — saving a board is an explicit persistence decision:
  - `Ephemeral` — resets on an engine rebuild (republish/save). A per-engine in-memory map.
  - `Session` — survives a republish (in-memory, board-scoped via the scheduler); clears on restart.
  - `Durable` — survives a republish **and** a server restart (a `node_state` store table; sqlite +
    postgres). Higher policies degrade to `Ephemeral` on a one-shot run that wires no backing.
- **Why it exists.** A save re-registers the board → rebuilds the engine → reset per-actor state, so
  a `trigger` re-fired its boot fire on every save. The `trigger` now declares `Session`, so its
  clock survives a save (no re-boot) but resets on restart — the intended behaviour, without the old
  process-global `static HashMap`. `write_point` keeps its coalescing memory as `Ephemeral`.
- **Still to do:** retained link values as the single source of truth (the engine already retains the
  latest per link — extend to a continuous hold once `watch`-driven); board-scoped cleanup of state
  on board delete.

### Stage E — Unify the live bus across the app
- Points page and Dashboards subscribe to the same SSE/zenoh-backed stream instead of 5s
  polling — one real-time substrate for points, board links, and widgets.

### Deferred
- G5 — write coalescing landed (no re-command of an unchanged value); writer *provenance* (board/run
  id) and slot *lease/expiry* still to do when the persistent writer is hardened.
- G6 (split the god-trait) — opportunistic.

### Remaining (UI / product decisions)
- Stage C — strip the "Continuous / On demand" run-mode dropdown down to Enabled + an optional
  advanced scan rate (a UX decision; the server already models it as enable + trigger).
- Editor polish — render the `quality` flag (color) and a freshness age on each node from the stream
  the editor now consumes (cosmetic; the data is already on the wire).
- Stage E — point the Points page and Dashboards at the same SSE/zenoh substrate instead of the 5s
  poll.

---

## Dependencies
```
Stage A (async seam + persistent net) ─┬─> Stage B (SSE) ──> Stage C (UX)
                                        └─> Stage D (component state) ──> Stage E (unified bus)
```
Stage A is the keystone; its one real unknown (reflow's long-lived-network behaviour) is now
spiked and confirmed (§2b). The async seam, the `BoardEngine`, and the interval scan wiring have
landed; what remains in Stage A is the `watch` primitive and scan-loop robustness (timeout, write
coalescing, `spawn_blocking`). Stages B–C deliver the visible win (real-time, no flicker, simple
UX); Stages D–E complete the Niagara/Sedona model.
