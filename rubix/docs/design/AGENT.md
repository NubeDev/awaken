# AGENT — AI agent runtime on the rubix substrate

Design for bolting an AI agent onto rubix as a **scoped principal**, reusing the
gate, capability grants, scoped-session reads, vector store, and audit/trace
substrate rather than importing a framework that brings its own. The **scope
authority** is [SCOPE.md](../SCOPE.md) ("AI-ready by construction"; "vectors live
beside the same data … so semantic search and agent memory need no separate
store") and its **"Two authz layers (do not conflate)"** rule; the crate
contracts are [STACK-DEISGN.md](../../STACK-DEISGN.md). Where this doc and
SCOPE.md disagree, SCOPE.md wins.

## Thesis

An agent needs five things — identity/permissioning, memory, tool calling,
"why did it act" provenance, and data access. Rubix already owns the load-bearing
four, with two honest caveats:

| Agent need | Rubix provides | Caveat |
| --- | --- | --- |
| Identity + permissioning | Scoped principal + capability grants (`rubix-gate`) | the `extension` identity kind exists; the full `rubix-ext` subsystem (JSON-RPC control, lifecycle) is **absent**, and agent-principal provisioning depends on it |
| Memory / semantic recall | SurrealDB vectors beside the data, queried via `rubix-query` | retrieval **and** persistence must run on the gate's seams (below), not an upstream side-connection |
| Provenance ("why did it act") | Audit + undo + trace, one correlation id | **commands only** — SCOPE audits mutations "(and opt-in sensitive reads)"; a read-only analyst is largely *not* audited unless reads are opted in |
| Data access | DataFusion unified surface (`rubix-query`) + scoped-session SurrealDB reads | native record reads are **not** a capability — they are row-perm scoped |
| Brain (LLM loop, providers, tool dispatch) | **— imported (Rig) —** | |
| Actuation (command a physical point) | **— not modeled —** | the [UI demo](../../ui-demo/rubix-v2/rubix-copilot/) is mostly an *actuator*; needs a fail-closed `device-actuate` grant + an egress wire (see "Actuator" below) |

So the design imports **the brain** (Rig) and wires its seams to the substrate
already built — and names one further gap the product demo exposes: **actuation**
is a third plane the gate does not yet guard (the "Actuator" section resolves the
grant; open question 8 tracks the egress). The agent is provisioned as a service-account principal, so its
*commands* are audited, undoable, and trace-correlated by construction, and it is
fenced to its granted rows (scoped session) and capabilities (the gate). The
data-plane safety story is that fence — **provided the memory and tool seams stay
on the gate's read/write paths** (the load-bearing assumption this doc makes
explicit, not a new sandbox).

## The two authz layers this design lives inside

SCOPE's non-negotiable split, restated because the agent straddles both:

- **Reads (incl. memory recall, semantic search, native record reads)** run on the
  gate-issued **scoped SurrealDB session**; SurrealDB row-level permissions decide
  what the agent sees. **Not a capability.**
- **Cross-plane actions** are app-enforced **capability grants** drawn from a closed
  vocabulary ([`Capability`](../../crates/rubix-gate/src/capability/kind.rs)). At the
  time this design was written it was five variants (`datasource-register`,
  `rule-invoke`, `ingest-publish`, `external-query`, `zenoh-subscribe`); the agent
  work below has since added `agent-memory-write`, `device-actuate`, and `rule-define`,
  among others. The registry is a **fail-closed allow-set** (WS-04): an unknown
  capability is denied, so every new agent surface is a deliberate enum change, never
  an assumed grant.

## Primary design — Rig brain + SurrealDB substrate

The brain is **Rig** (`rig-core`, `0xPlaygrounds/rig`): a mature Rust LLM/agent
library (20+ providers incl. Ollama, a `VectorStoreIndex` trait, a `Tool` trait,
official Rust MCP SDK support). **Two** of its four seams are genuinely
rubix-specific work — `Tool` and `Memory` — because both must cross the gate;
the other two are thin.

```
        ┌──────────────────────── rubix-agent (new crate) ───────────────────────┐
        │                                                                         │
        │   Rig agent loop  ──Provider──►  LLM (edge: Ollama / cloud: Claude)      │
        │        │                                                                 │
        │        ├──Memory (impl VectorStoreIndex IN rubix-agent)                  │
        │        │      reads  ──► rubix-query::search on the SCOPED SESSION        │
        │        │      writes ──► gate command (mutation crosses the gate)         │
        │        │                                                                 │
        │        ├──Tool ──► CAPABILITY BRIDGE ──► rubix-gate                       │
        │        │      external-query · rule-invoke (covers insight recording)     │
        │        │      · zenoh-subscribe — each tool maps to a real grant          │
        │        │                                                                 │
        │        └──Channel ──► rubix-bus LIVE-QUERY plane (row-perm scoped wake)   │
        │               (NOT the in-process plane — see "Two wake paths")           │
        │                                                                         │
        │   provisioned as a SCOPED PRINCIPAL ── row perms + grants decide reach    │
        └─────────────────────────────────────────────────────────────────────────┘
```

### Seam mapping

| Rig seam | Backed by | Nature |
| --- | --- | --- |
| `Provider` | Rig's provider set; profile-driven (edge local / cloud remote) | config wiring |
| `Channel` | `rubix-bus` **live-query** plane (row-perm scoped) | thin adapter |
| `Memory` (`VectorStoreIndex`) | **implemented in `rubix-agent`** over `rubix-query::search` on the scoped session (reads) and a gate command (writes) | **rubix-specific seam** |
| `Tool` | **capability bridge → `rubix-gate`** | **rubix-specific seam** |

Both substantial seams are ours because both must honor the gate. `rig-surrealdb`
is **not adopted wholesale**: it opens its own SurrealDB connection, which would
let memory reads escape row-perm scoping and memory writes escape the gate (no
audit, no correlation id, no undo) — collapsing the safety thesis. Implementing
`VectorStoreIndex` directly over `rubix-query` + the scoped session keeps recall
on the gate's read path and persistence on its write path, and is likely *less*
work than forcing the upstream crate to accept an injected session. `rig-surrealdb`
remains a reference for the SurrealQL vector-search shape only.

### Memory writes cross the gate

Storing working/episodic memory and its embeddings is a **mutation**, so contract
#1 requires it to cross the gate — the same rule insights already follow
([evaluate/record.rs](../../crates/rubix-rules/src/evaluate/record.rs) builds a
`Command` and drives `apply`). A gate `Command` is always constructed *with a capability*, and `apply` authorizes
that grant before any write — there is **no generic "write any record" path**
([authorize.rs](../../crates/rubix-gate/src/command/authorize.rs) runs first, fail
closed). None of the five existing variants names "write agent memory" (`rule-invoke`
is for recording a rule decision, not arbitrary memory persistence). So memory-write
has exactly **one honest answer: add a fail-closed `agent-memory-write` `Capability`
variant.** "Classify it as an existing data-plane path" is not implementable as
written — every write must name a capability, and none of the five fits, so that
branch collapses into "invent a capability" anyway. **"Import nothing" therefore
understates it** — the memory *schema* is borrowed, but the write path is real,
gated code behind a new grant.

### Analyst vs. operator, in the real two-layer model

One crate, one runtime. The split is **row perms first, one capability second** —
not a bag of invented grants:

- **Analyst** — a principal whose **row permissions** scope its reads (semantic
  recall + native queries on the scoped session). Holds **`external-query`** *only*
  if it must reach the DataFusion/Postgres plane. Read-only "ask your data."
  Recording memory of what it read still crosses the gate (above).
- **Operator** — analyst **plus `rule-invoke`**, which already covers recording an
  insight (the insight write *is* a `rule-invoke` gate command). It does not need a
  separate "write-insight" grant.

Promoting analyst → operator is granting `rule-invoke`, not a rebuild. **No
code-execution tool is in scope**, so no microVM/WASM sandbox is required; if an
arbitrary-code tool is ever added, isolate *that tool* behind an optional cargo
feature, not the whole runtime.

### Actuator: the operator tier the demo actually needs

The [Rubix Copilot UI demo](../../ui-demo/rubix-v2/rubix-copilot/) is the product
target, and reading it back changes the operator story above. Its conversational
surface is *mostly an operator that changes the building*: the headline turns end
in **actuation** — "Apply pre-cool to L4 West", "Fail over to backup CRAC",
"Restart gateway GW-02", "Arm battery to discharge 22 kW at 2:30pm", "Schedule the
night profile on Level 5". The read-only Q&A (demand split, worst zones, week vs.
week) is the *minority* of the demo. So the demo lives in a tier this doc did not
fully name: an agent that **commands physical points**, not just records insights.

That tier has **no gate path today**, and this is the load-bearing gap to close
before the demo is buildable:

- The five capabilities ([`Capability`](../../crates/rubix-gate/src/capability/kind.rs))
  are `datasource-register`, `rule-invoke`, `ingest-publish`, `external-query`,
  `zenoh-subscribe`. **None is "command a device."**
- `rule-invoke` does **not** actuate. A rule's script returns a
  [`Decision`](../../crates/rubix-rules/src/engine/decision.rs) (`fired` / `value`
  / `reason`) that is recorded as an **append-only insight record** via the gate
  ([record.rs](../../crates/rubix-rules/src/evaluate/record.rs)). It writes a row;
  it does not move a setpoint or a relay.
- A gate [`Command`](../../crates/rubix-gate/src/command/action.rs) mutates a
  SurrealDB **record** (`Create`/`Update`/`Delete` over JSON). It has no egress to
  a Modbus/BACnet/Zenoh control point either.

So actuation is a *third plane* the gate does not yet guard, and "operator =
analyst + `rule-invoke`" understates the demo. **Decision: add a deliberate,
fail-closed `device-actuate` `Capability` variant** rather than overloading
`rule-invoke`. The rationale follows the same logic AGENT.md already applies to
memory-write (open question 3b):

- **Why not overload `rule-invoke`.** `rule-invoke` is semantically "evaluate a
  rule and record its decision" — an *append-only insight*. Conflating it with
  "drive a physical output" would mean granting an analyst the power to record an
  insight *also* silently grants the power to actuate hardware. That collapses the
  fail-closed allow-set's whole point: a new agent surface must be a deliberate
  enum change, never an assumed grant (WS-04). Pre-cooling a floor and recording
  "the floor is warm" must be **two grants**.
- **Why a new variant, not a data-plane write.** Actuation is *cross-plane* by
  definition — it leaves SurrealDB and reaches a device — which is exactly the
  test for the second authz layer (contract #2). But the egress is **not** driven
  from the gate's `apply` step: `apply` is a closed pipeline (authorize → validate →
  correlate → capture → audit, [apply.rs](../../crates/rubix-gate/src/command/apply.rs))
  whose only side effect is a SurrealDB record write plus an immutable audit row. It
  has **no device I/O hook**, and adding one would bury Modbus/BACnet inside the
  record-write path or audit *intent* as if the device had obeyed. The honest model
  is an **effect-record + egress-worker + ack**, reusing the live-query plane this
  doc already mandates for wakeups:
  1. `device-actuate` writes a **desired-effect record** through the gate — grant
     checked, correlation id minted, before/after of the *effect record* captured,
     audit row appended. This is exactly what the gate does well today.
  2. A **device-egress worker** subscribes to effect records via the live-query
     plane ([livequery/subscribe.rs](../../crates/rubix-bus/src/livequery/subscribe.rs)),
     performs the physical I/O (Modbus/BACnet/Zenoh), and writes an **ack/result
     record** carrying the same correlation id.
  3. The ack closes the provenance loop the demo's "why did Rubix act" panels imply:
     intent, command, and device outcome are three correlated rows, not one optimistic write.
- **Undo is record-undo, not device-undo.** The gate captures before/after of the
  *record* ([capture.rs](../../crates/rubix-gate/src/command/capture.rs)); reverting
  it does **not** move a setpoint back. Physical reversal (a reverse setpoint where
  the device supports it) is the egress worker's job, issued as a *new* effect, not a
  gate rollback. Keep the two undos distinct in any wiring.
- **Tiering, restated.** **Analyst** = scoped reads (+ `external-query` for the
  Postgres/DataFusion plane). **Operator** = analyst + `rule-invoke` (record
  insights). **Actuator** = operator + **`device-actuate`** (command points). The
  demo's Avery is an *actuator*; promoting through the tiers is granting one
  variant at a time, fail closed at every step.
- **Scope boundary.** `device-actuate` is **not** an arbitrary-code tool — it
  commands a *registered point* through a typed effect (setpoint offset, relay
  state, mode select), so it needs no sandbox. The actual egress (Modbus/BACnet via
  the device layer, or a Zenoh control key) is a transport concern; the *grant and
  audit* are the gate's, the *wire* is the device/ingest plane's. Adding the
  variant is a registry change in `rubix-gate`; wiring the egress is follow-on work
  in the device/actuation layer, which `STACK-DEISGN.md` does not yet name as a
  crate — flag that as a dependency (open question 8).

### Demo action → capability/command mapping (the agent's tool manifest)

Every interactive surface in the demo, mapped to the gate path that must back it.
This is the agent's **tool manifest**: each Rig `Tool` the agent exposes is one row
here, fronted by the capability bridge so the LLM can never reach a plane the
principal was not granted. Rows marked **capability absent** require a deliberate,
fail-closed registry change before they are buildable.

| Demo action (source) | Plane | Backing path | Capability | Capability state |
| --- | --- | --- | --- | --- |
| "Why is demand high?", "worst zones", "this week vs last" ([answers.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/answers.js) `demand`/`zones`/`compare`) | read | `rubix-query` + scoped session | none (row perms) · `external-query` only if it crosses to Postgres/DataFusion | present |
| Attention queue / "2 things need you" wake ([data.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/data.js) `RX.moments`) | read | `rubix-bus` **live-query** plane (row-perm scoped) | none (scoped session) | present |
| "Why did Rubix act" / insight provenance | read | audit + correlation id over the gate | none (the command already wrote it) | present |
| Record an insight / "log this" (the `moments` themselves are rule firings) | command | `record_insight` → gate `Command` | `rule-invoke` | present ([record.rs](../../crates/rubix-rules/src/evaluate/record.rs)) — but see free-form caveat below |
| "Pin to Overview", "Watch the chillers for an hour" ([answers.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/answers.js) `compose`/`pin`) — persist a board + its embedding | command (memory-write) | `VectorStoreIndex` write → gate `Command` | **`agent-memory-write`** | present ([kind.rs](../../crates/rubix-gate/src/capability/kind.rs)) |
| "Apply pre-cool to L4 West" ([answers.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/answers.js) `precool`) — setpoint offset | actuate | `device-actuate` effect record → egress worker → ack | **`device-actuate`** | capability present ([kind.rs](../../crates/rubix-gate/src/capability/kind.rs)); **egress worker absent** |
| "Fail over to backup CRAC" (`failover`) — mode/relay select | actuate | `device-actuate` effect record → egress worker → ack | **`device-actuate`** | capability present ([kind.rs](../../crates/rubix-gate/src/capability/kind.rs)); **egress worker absent** |
| "Restart gateway GW-02", "Arm battery to discharge" ([data.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/data.js) `ahu`/solar answer) | actuate | `device-actuate` effect record → egress worker → ack | **`device-actuate`** | capability present ([kind.rs](../../crates/rubix-gate/src/capability/kind.rs)); **egress worker absent** |
| "Roll out night profile to Level 5", "schedule" ([answers.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/answers.js) `savings`) — write/enable a rule binding | command (rule config) | gate `Command` over the rule registry record | **`rule-define`** (NOT `rule-invoke`) | **capability absent** |
| "Draft this week's report", "Send to board" (`report`) — outbound export/email | outbound | **not `rubix-agent`** — `rubix-server`/`rubix-ext` transport | n/a here | out of scope here — `rubix-ext` transport absent |
| ⌘K palette → "ask Rubix" (free-form) ([app.js](../../ui-demo/rubix-v2/rubix-copilot/copilot/app.js) `RX.openPal`) | read | routes into the read/command tools above | per resolved intent | present |

Reading the manifest top to bottom: the demo's **analyst spine is present** in the
substrate, its **insight-recording path exists**, and the three deliberate grants its
wow-factor needs — `agent-memory-write`, `device-actuate`, `rule-define` — **have all
since landed** ([kind.rs](../../crates/rubix-gate/src/capability/kind.rs)). What
remains for actuation is the **egress worker** (effect record → physical I/O → ack),
plus the Rig brain crate and the out-of-scope outbound transport.
Nothing in the actuate rows is buildable until `device-actuate` exists *and* a
device-egress path (effect record → worker → ack) is named.

**Free-form insight caveat.** `record_insight`
([record.rs](../../crates/rubix-rules/src/evaluate/record.rs)) takes a *rule* and a
`Decision` (`fired`/`value`/`reason`) and writes the decision as the insight. A
free-form agent "log this" has no rule and no `Decision` to supply, so the
"record an insight" row only maps to `rule-invoke` if the agent invokes a real rule.
A free-form agent note is a *different* shape and likely belongs under
`agent-memory-write` (or a dedicated note record), not `rule-invoke`. Resolve before
treating the row as ready.

### Two wake paths (the Channel seam must pick the scoped one)

`rubix-bus` has two planes and they differ in scoping:

- **Live-query plane** — row-perm scoped on the principal's session; the agent
  sees only firings it is permitted to read. **The agent must wake on this plane.**
- **In-process control plane** — a tokio broadcast where `subscribe(bus,
  event_type)` takes **no principal**
  ([inprocess/subscribe.rs](../../crates/rubix-bus/src/inprocess/subscribe.rs)); it
  is ungated and unfiltered. The agent must **not** use it to wake on insights, or
  it would observe firings outside its grant.

### Integration tools — MCP, inbound only here

Rig's official MCP support carries both directions, but they belong in different
crates to avoid duplicating "rubix-over-the-gate to outside callers":

- **Inbound (owned by `rubix-agent`)** — consume external MCP servers as agent
  tools, each fronted by the capability bridge so an external tool call is still
  gated.
- **Outbound (NOT here)** — exposing rubix capabilities *as* MCP/JSON-RPC tools for
  an external agent to drive rubix is a **transport** concern that overlaps the
  planned `rubix-ext` JSON-RPC control plane / `rubix-server`. Putting it in
  `rubix-agent` too would be the cross-crate duplication CLAUDE.md/SCOPE push back
  on. Keep it in the server/ext layer.

### Memory schema — borrow the taxonomy, import nothing

Rubix is on `surrealdb = "3"`, whose engine is tuned for agent memory, so the
store suffices. Memory tables lay out on the existing tag graph (`record → tagged
→ tag` is already a knowledge graph). The memory *taxonomy* (working / semantic /
episodic / procedural / preference / shared) and the knowledge-graph-plus-vector
retrieval patterns are **schema references** from SurrealDB's `agent-memory` demo
(Python) and the Spectron memory model (closed-source) — design input only, no
dependency. The retrieval seam is the `rubix-agent` `VectorStoreIndex` impl over
`rubix-query`, which today searches with `vector::distance::euclidean`
([search/nearest.rs](../../crates/rubix-query/src/search/nearest.rs)). There is no
cosine path, but this is **not** a model-choice constraint: on **L2-normalized**
vectors euclidean ranking is monotonic with cosine (identical nearest-neighbour
order), so the only requirement is to normalize embeddings before insert — which
works with OpenAI/Claude/most models.

### Load-bearing contracts honored

- **#1 two enforcement points** — agent *commands* (memory-write, insight via
  `rule-invoke`) cross the gate; agent *reads* (recall, search) run on the scoped
  session. The Memory seam is built to keep this true.
- **#2 two authz layers** — record reads via the scoped session + row perms;
  cross-plane use (`external-query`, `rule-invoke`, `zenoh-subscribe`) via grants.
  No read is mislabeled as a capability.
- **#3 correlation id** — minted at the gate for each agent command, threaded into
  insight + event + spans.
- **#6 SurrealDB does as much as possible** — memory is SurrealDB-native vectors;
  no second store (honors the "no second store for vectors" non-goal).

## Alternative to consider — IronClaw (NearAI)

`nearai/ironclaw` is the closest worked example of this shape — a **reference** and
a **fallback adoption path**.

**What it is.** A security-focused agent OS in Rust that **builds on `rig-core`**
(confirmed: `crates/ironclaw_llm` depends on `rig`) and wraps it with: pgvector +
RRF hybrid memory, a WASM (`wasmtime`) / Docker (`bollard`) tool sandbox, AES-GCM
"zero-exposure" secret injection at the host boundary, MCP support, and
multi-channel I/O (Telegram/Discord/webhooks).

**Why it validates the primary design.** A team whose entire pitch is hardening did
**not** reinvent the agent loop — they took Rig and wrapped it. Rubix is the same
equation with a different wrapper; for every layer IronClaw built, rubix already
owns a better-integrated one.

| Layer | IronClaw | Rubix (primary design) |
| --- | --- | --- |
| Brain | Rig | Rig (same) |
| Memory | pgvector + RRF | `VectorStoreIndex` over `rubix-query` on the scoped session, writes through the gate — existing store, no new DB |
| Tool security | bespoke safety layer + WASM/Docker | gate + the five capability grants (already built) |
| Secrets | AES-GCM inject at host boundary | gate brokers data → agent sees scoped results, never raw creds |
| Channels | Telegram/Discord/… | `rubix-bus` live-query plane |

**When to reach for it.** If the Tool-bridge + Memory-seam work proves larger than
expected, IronClaw is a same-brain (Rig) head start whose patterns drop in with the
storage and sandbox swapped out. **Why not adopt wholesale:** it brings pgvector
(collides with "SurrealDB is the only store") and a code-exec sandbox (out of scope
here), and it is an *assistant OS*, not a data-platform component — its identity and
channel model is not rubix's principal model. Lift the patterns
(provider-chain-with-failover init; RRF hybrid search, reproducible natively with
SurrealDB full-text + vectors; the secret-discipline rule, enforced by the gate),
not the binary.

## Rejected / not-a-fit

- **ZeroClaw** — standalone hand-rolled loop, marketing-stage maturity, no
  SurrealDB memory; nothing it offers that Rig doesn't, with less adoption.
- **Smooth (SmooAI)** — orchestration platform for *coding* agents with a microVM
  sandbox; wrong altitude and the sandbox is out of scope.
- **Kowalski** — Rig-adjacent but pgvector + Apache AGE memory; same store conflict
  as IronClaw, less mature.
- **Spectron / Surrealism** — SurrealDB-native but **memory layers, not brains**
  (Spectron is closed-source; even SurrealDB punts the orchestration loop to an
  external runtime). They overlap with the provenance substrate rubix already owns;
  used here as schema references only.

## Crate placement

A new `rubix-agent` crate in the workspace crate map:

| Crate | Role | Owns |
| --- | --- | --- |
| `rubix-agent` | AI agent runtime | Rig agent loop; `Provider`/`Channel` adapters; the `Memory` (`VectorStoreIndex`) impl over `rubix-query` + scoped session + gate writes; the `Tool` → capability bridge; agent-principal provisioning; **inbound** MCP |

Depends on: `rubix-core` (principal, correlation id), `rubix-gate` (capability
dispatch, audit, scoped session), `rubix-store`/`rubix-query` (vectors +
DataFusion), `rubix-bus` (live-query channel), `rubix-rules` (the `rule-invoke`
tool). Edge default builds the local provider path; cloud-only providers sit
behind the `cloud` feature and fail closed when absent.

## Open questions

1. **Edge LLM story.** Local model via Ollama for offline operation vs.
   agent-is-cloud-only with edge running rules-only. Drives the `Provider` profile
   and degraded-mode behavior when no model is reachable.
2. **Rig version + license pin.** Confirm exact license on `rig-core` (lib.rs
   reported ambiguity) and that its `VectorStoreIndex`/`Tool` trait shapes are
   stable enough to implement against, before pinning (production-ready-only rule).
3. **Memory seam over the gate (the safety thesis).**
   - **3a** — Can the `VectorStoreIndex` impl be driven entirely on the gate's
     scoped session for reads (confirmed feasible via `rubix-query`), and what is
     the exact write path?
   - **3b** — *Decided:* memory-write is a new `agent-memory-write` `Capability`
     variant. There is no generic record-write path — every gate command authorizes a
     named capability first ([authorize.rs](../../crates/rubix-gate/src/command/authorize.rs)),
     and none of the five fits — so "reuse an existing path" is not implementable.
     The only open part is the exact write payload (memory record + embedding schema).
   - **3c** — *Resolved:* `rubix-query` uses `vector::distance::euclidean`. This is
     not a model constraint — L2-normalize embeddings before insert and euclidean
     ranking equals cosine ranking. Action: normalize on write; document it.
4. **Tool capability surface.** Three new fail-closed variants fall out of the demo
   manifest: **`device-actuate`** (actuate rows), **`agent-memory-write`** (watch/pin
   rows, 3b), and **`rule-define`** (write/enable a rule binding — `rule-invoke` only
   *records a decision*, it cannot mutate a rule definition/binding/schedule). Still
   open: does any *inbound MCP* tool need a per-tool grant beyond these, or do they
   all fold into `external-query`? Propose any addition as a deliberate registry change.
5. **Auditing read-only analysis.** Audit is command-scoped, so an analyst agent's
   reads leave no trail. For an *AI* analyst this is closer to a requirement than an
   option: "why did the agent conclude X" is unanswerable without recording what it
   read. Decide whether to opt analyst reads into sensitive-read auditing — lean yes.
6. **`rubix-ext` dependency — a build blocker, not just a question.** The agent is
   provisioned as a service-account principal, which leans on the
   extensions-as-principals model; `rubix-ext` is absent. The `extension` principal
   *kind* exists, but how the agent principal is created and granted capabilities
   today is unresolved — and it gates *every* line of agent code, not a later tier.
   Decide the minimum provisioning path before starting `rubix-agent`.
7. **Spectron upstreaming.** Track which memory primitives SurrealDB upstreams into
   the open engine over time, to avoid building what becomes native.
8. **Device-actuation egress (the demo's wow-factor).** `device-actuate` gives the
   *grant + audit*; it still needs a *wire*, and that wire is **not** the gate's
   `apply` step (a closed write+audit pipeline with no device hook). The model is an
   **effect record → egress worker (subscribes via the live-query plane) → ack
   record**, all sharing one correlation id (see Actuator section). `STACK-DEISGN.md`
   names no device/actuation crate yet (ingest is subscribe-only). Decide where the
   egress worker lives and whether physical reversal (a new reverse-setpoint effect,
   *not* a gate undo) is in scope, before the pre-cool/failover/restart actions ship.
