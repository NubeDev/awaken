# Fleet & Identity — hackline + ca-server integration

Design for how rubix reaches two **external NubeIO services** — `hackline`
(Zenoh-native fleet / remote-access) and `ca-server` (a Vault-backed certificate
authority) — from both the edge and cloud profiles, through purpose-built Rust
client crates, with rubix UI to manage them later. Reads against
[SCOPE.md](../SCOPE.md): *"One binary, edge to cloud"* (§1), *"Everything is a
scoped principal"* (§5), *"Commands go through the gate; reads are SurrealDB-native"*
(§7), and the **Edge↔cloud** open question (§"Sync and conflict model", Open
Question 1). It sits beside [`rubix-sync`](../../crates/rubix-sync) — the data-plane
shipper rubix already runs over Zenoh.

The two services live at `bin/hackline` (Rust) and `bin/ca-server` (Go). Both are
built **standalone**; rubix is a consumer, not an owner.

> **Source of truth.** hackline carries a `bin/hackline/INTEGRATION-RUBIX.md` that
> describes a *rubix consumption contract* — **it is stale** (it predates the current
> `rubix/crates/rubix-*` layout and the auth model below). This document supersedes
> it for every rubix-side decision. The authoritative hackline sources are its own
> [`SCOPE.md`](../../../bin/hackline/SCOPE.md),
> [`DOCS/ARCHITECTURE.md`](../../../bin/hackline/DOCS/ARCHITECTURE.md),
> [`DOCS/AUTH.md`](../../../bin/hackline/DOCS/AUTH.md),
> [`DOCS/CERT-RENEWAL.md`](../../../bin/hackline/DOCS/CERT-RENEWAL.md), and the
> `hackline-client` / `hackline-proto` crate surfaces — all verified for this doc.

## What the two services are

### hackline — the fleet / remote-access plane

Two planes on one Zenoh fabric (`bin/hackline/SCOPE.md`; phases 0–5 **done** per
`bin/hackline/CLAUDE.md`):

- **Tunnel plane** — per-device HTTP and TCP, reachable from the cloud. A device's
  own loopback service (i.e. rubix's existing REST surface) becomes reachable at
  `https://device-N.cloud.com/` without the device opening an inbound port. The
  agent serves a queryable `hackline/<org>/<zid>/tcp/<port>` and bridges to
  `127.0.0.1`. **Every keyexpr is `hackline/<org_slug>/<zid>/…`** — the `<org>` tenant
  prefix is mandatory (`hackline-proto/src/keyexpr.rs`) and is what the gateway ACL
  scopes; any rubix topic, test, or "done test" assertion must include it.
- **Message plane** — typed `MsgEnvelope`s: **events** (device→cloud, SSE-fanned at
  the gateway), durable **commands** (cloud→edge, at-least-once), **api** RPC
  (request/response over a queryable), and logs.

One cloud `hackline-gateway` (axum REST `/v1/*` + SSE + TCP listeners + SQLite +
Zenoh peer), one device-side `hackline-agent` **binary**, the device SDK
`hackline-client`, and the wire-types crate `hackline-proto`. The gateway is a single
privileged Zenoh principal — *"a compromised gateway owns every device's loopback,
by design"* (AUTH.md). Auth on the message plane is **Zenoh ACL on the session
itself; the SDK opens no second auth layer** (SCOPE §8.2).

### ca-server — the certificate authority under the fabric

A Vault-backed CA (`bin/ca-server/API.md`, `README.md`):

- **Vault PKI** — a 60-year `NubeIO CA` root; `POST /ca/sign` issues a device
  certificate from a CSR; `GET /ca/certificate` serves the root for trust anchoring.
- **HMAC-SHA256 device auth** — every request carries `X-GlobalUUID`, `X-Timestamp`,
  `X-HMAC = HMAC(globalUUID + timestamp + presharedSecret, key=presharedSecret)`,
  base64, within a ±5-minute replay window. The preshared secret is never on the wire.
- **Admin surface** — `POST /devices/register` (the **caller supplies**
  `global_uuid` + `preshared_secret`; the server stores an encrypted copy and returns
  **no secret** — so whoever registers must generate, surface, and hand off the secret
  itself), `POST /ca/revoke`, `GET /admin/devices[/search]`, gated by the special
  `X-GlobalUUID: admin` principal. Only `/health` and `/ca/certificate` are
  unauthenticated; everything else requires device HMAC, and the admin routes require
  the `admin` principal (`routes/routes.go`).

ca-server is the **root of trust under the Zenoh fabric**: the X.509 cert it issues is
what each device (and the gateway) presents for **mTLS on Zenoh** (SCOPE §3.5,
mTLS/QUIC; ZID + per-device keypair + ACL). Revocation = pulling the ACL/cert.

## The key fact: hackline already integrates ca-server

This is the load-bearing discovery and it sets rubix's scope. **hackline already
obtains and renews its Zenoh certs from ca-server, automatically** — rubix does *not*
build edge PKI.

Both `hackline-agent` and `hackline-gateway` carry a `[renewal]` config block and a
renewal loop (`hackline-agent/src/renewal.rs`, `hackline-gateway/src/zenoh_renewal.rs`,
`DOCS/CERT-RENEWAL.md`, `DOCS/CA-MANUAL.md`):

1. **Before the Zenoh session opens** — no key → generate RSA keypair; no cert →
   build a CSR, HMAC-authenticate to ca-server's `/ca/sign`, write the signed cert.
2. **Background** — near expiry, re-CSR the *same* key, re-sign, write, exit; systemd
   `Restart=always` brings the process back on the fresh cert in ~2s. The key is
   preserved across renewals.
3. `global_uuid` = the cert CN = the identity registered at ca-server;
   `preshared_secret` is the one from CA registration.

The CA endpoint in hackline's own deployment is `https://hackline.ca.nube-iiot.com/ca/sign`
— the same `/ca/sign` + HMAC surface `bin/ca-server` documents. **The identity →
fabric leg is already solved by hackline.** rubix rides it.

## How the pieces compose for rubix

```
   ┌────────────────────────────────────────────────────────────────────────┐
   │  IDENTITY (done by hackline)        FABRIC            FLEET (rubix uses) │
   │                                                                          │
   │  ca-server ──cert──► hackline-agent  ──► Zenoh mTLS ──► hackline message │
   │  (Vault PKI)         renewal loop         fabric          + tunnel planes│
   │     ▲  HMAC /ca/sign      writes cert        ▲               ▲           │
   │     │                  /etc/hackline/certs   │ Arc<Session>  │ ClientSession
   │     │ admin: register/  ─────────────────────┼───────────────┼────────── │
   │     │ revoke/list                            │               │           │
   │  ┌──┴───────────────────────────────────────┴───────────────┴─────────┐ │
   │  │                      RUBIX  (edge or cloud)                          │ │
   │  │   gate + capabilities · rubix-sync (data) · rubix-server REST · UI   │ │
   │  └──────────────────────────────────────────────────────────────────────┘
   └────────────────────────────────────────────────────────────────────────┘

   CLOUD: ca-server + Vault · hackline-gateway · rubix role:cloud (fleet + CA UI)
   EDGE : hackline-agent (separate binary, owns cert+fabric) · rubix role:edge
```

The division of labour, stated so rubix never duplicates a neighbour:

- **ca-server owns *who you are*** — issues the X.509 identity.
- **hackline owns *how the cloud reaches you*** — obtains/renews that identity for
  the fabric (its `[renewal]` loop), runs the agent/gateway, carries events/cmd/api
  and the HTTP/TCP tunnels.
- **rubix owns *what you do*** — opens a Zenoh session **with the cert hackline
  already manages**, participates in the message plane, ships data via `rubix-sync`,
  and (cloud-side) *manages* the fleet and the CA through their admin surfaces.

So rubix's two crates are deliberately **thin and non-duplicative**: a message-plane
consumer and a cloud management client. Neither re-implements cert bootstrap or
renewal.

## The Rust crates rubix adds

Two new crates, each a **scoped principal reaching one external plane** — the SCOPE
§5 model the gate already enforces for extensions. They join the workspace beside
`rubix-sync`.

| Crate | Wraps | Plane | Profile |
| --- | --- | --- | --- |
| **`rubix-fleet`** | `hackline-client` + `hackline-proto` (only) | message plane: events / cmd / api | edge + cloud |
| **`rubix-ca`** | ca-server **admin** REST over HTTP + HMAC | register / revoke / list (management) | cloud (read-only edge) |

### `rubix-fleet` — a `ClientSession` over a shared Zenoh session

`hackline-client` does **not** open Zenoh; it wraps a session the host already opened:
`ClientSession::from_session(Arc<zenoh::Session>, org, zid)`. The *target* end state is
one shared session:

> **Target: rubix opens one `zenoh::Session` per process and shares the `Arc` between
> `rubix-sync` (data plane) and `rubix-fleet` (control plane).** Same fabric, same
> cert, one set of peers; the mTLS identity is the cert `hackline-agent`'s renewal
> loop already wrote to `/etc/hackline/certs/…`, with Zenoh's TLS config pointed at
> those paths rather than minting its own (see Open Question 1).

**This is a required refactor, not the current state.** `rubix-sync` already accepts
an `&Session`, but `rubix-ingest` opens its *own* session
(`subscribe/listen.rs::open_subscription` calls `zenoh::open`), and `rubix-server`'s
`main.rs` owns no shared session today. Hoisting session ownership into `main` (or an
`AppState` field) and threading the `Arc` to `rubix-sync` / `rubix-ingest` /
`rubix-fleet` is **part of the Step-1 work**, tracked as Open Question 2.

```rust
/// rubix's participation in the hackline message plane. Wraps a concrete
/// ClientSession; it does NOT abstract the transport behind a trait — hackline's
/// type IS the stable boundary (contrast CONTROL-ENGINE.md, where the engine's
/// churning wire format is what justifies a trait).
pub struct Fleet {
    session: Option<hackline_client::ClientSession>, // None == offline
    gate: Arc<rubix_gate::Gate>,
}

impl Fleet {
    /// Built from the shared Zenoh session AFTER it is open. `org`/`zid` are the
    /// tenant slug and Zenoh ID this session publishes under — they must match a
    /// gateway ACL entry (see "ZID + ACL provisioning" below). Offline short-circuits
    /// before the Zenoh session is opened at all, so a network-less edge never blocks
    /// on peer discovery.
    pub fn new(zenoh: Arc<zenoh::Session>, org: &str, zid: Zid, gate: Arc<rubix_gate::Gate>) -> Self;

    /// Each verb is a gate-checked capability, then a thin pass-through. Bodies are
    /// `serde_json::Value` — the SDK wraps them in a `MsgEnvelope`; it is not a raw
    /// byte channel (`hackline-client::ClientSession::publish_event`).
    pub async fn publish_event(&self, p: &Principal, topic: &str, body: serde_json::Value) -> Result<()>; // fleet-publish
    pub async fn serve_api(&self, p: &Principal, topic: &str, h: ApiHandler) -> Result<()>;               // fleet-serve
    pub async fn subscribe_cmd(&self, p: &Principal, topic: &str) -> Result<CmdStream>;                   // fleet-subscribe
}
```

> **ZID + ACL provisioning is a Step-1 prerequisite, not just id hygiene.** The
> gateway authorizes by binding a **cert CN → a single `hackline/<org>/<zid>/**`
> subtree** (one ACL file per device: `{ zid, cert_common_name }`,
> `hackline-gateway/src/config.rs`). So reusing the agent's cert is **not** sufficient
> for rubix to publish: rubix's session has its *own* ZID, and either it must publish
> under the agent's ZID/org or the gateway needs an ACL entry granting rubix's ZID
> under that CN. Until rubix's `(org, zid, cert CN, ACL entry)` tuple is decided and
> provisioned, Step 1 cannot pass its done test. This is the hard edge of Open
> Question 1/3 — resolve it before writing the publisher, not after.

Constraints, each independently justified (not borrowed from the stale doc):

- **Dependency firewall.** `rubix-fleet/Cargo.toml` depends on `hackline-client` and
  `hackline-proto` **only** — never `hackline-core`, `-agent`, or `-gateway`. The
  hackline crate table (`DOCS/ARCHITECTURE.md`) marks those three as server/internal,
  not the consumable SDK; a transitive pull of any of them is a smell. Enforce in
  rubix CI via `cargo metadata` (catches transitive), not text grep. Path-dep now;
  semver-pin once hackline publishes.
- **Offline = no Zenoh session.** `--offline` / `fleet: null` skips opening Zenoh
  entirely; `Fleet::session` is `None`, every call a no-op/err. This is also why the
  data and control planes share the session — one offline switch, not two.
- **`api`/`cmd` are Phase 2 in the SDK**; `publish_event`/`publish_log` exist today
  (Phase 1.5). rubix's first seam (events) needs nothing new from hackline.
- **Persistent cmd dedupe.** Durable commands are at-least-once; a crash between
  handler-success and ack re-delivers a `cmd_id`. Dedupe in SurrealDB (not an
  in-memory map), TTL exceeding the gateway's command TTL.

**Message plane needs no bearer token** — it is authed by Zenoh ACL on the session
(AUTH.md / SCOPE §8.2). Bearer tokens are for the *gateway REST surface*, used only by
the cloud management client below.

### `rubix-ca` — ca-server's admin surface, for management not enrollment

Edge enrollment and renewal are hackline-agent's job (above), so `rubix-ca` is **not**
an edge-PKI crate. It is a **cloud-only** thin client of ca-server's **admin** API that
powers the management UI. There is no safe "read-only edge" mode: ca-server exposes
only `/health` and `/ca/certificate` without authentication, and every device-status /
listing route is admin-HMAC-gated — so an edge would have to hold the `admin` secret to
read status, which it must not. If per-device cert status is ever wanted on edge, it
needs a *new* device-scoped status endpoint in ca-server, not the admin API.

```rust
/// rubix's CLOUD management client of ca-server. ca-server is Go with no SDK, so the
/// boundary is rubix's to define — a small internal trait with one impl is fine here.
pub struct CaAdmin {
    base_url: Url,
    secret: HmacSecret,        // the "admin" principal's preshared secret — cloud only
    gate: Arc<rubix_gate::Gate>,
}

impl CaAdmin {
    /// ca-server's `/devices/register` does NOT mint a secret — the caller supplies
    /// `global_uuid` + a preshared secret it generated. So rubix generates the secret,
    /// submits it, and returns it to the operator to hand off to the device (it is
    /// shown once and never retrievable from ca-server afterwards).
    pub async fn register_device(&self, p: &Principal, uuid: &str) -> Result<GeneratedSecret>; // ca-admin
    pub async fn revoke_device(&self, p: &Principal, uuid: &str) -> Result<()>;                // ca-admin
    pub async fn list_devices(&self, p: &Principal, page: Page) -> Result<Vec<DeviceRow>>;     // ca-admin
}
```

It owns HMAC-SHA256 request signing (the ±5-minute window) and the `admin` preshared
secret as a **cloud secret at rest**. The enrollment ceremony: rubix **generates** the
device's preshared secret client-side, registers `(global_uuid, secret)` with
ca-server, and hands the secret to the device's hackline-agent (which then signs CSRs
with it) — rubix is the *start* of the ceremony, ca-server stores only an encrypted
copy, and hackline finishes it. A lost secret means re-register, since ca-server never
returns it.

### Both are capabilities at the gate

Reaching another plane is an **app-enforced capability grant** (SCOPE §"Two authz
layers"), not a SurrealDB row permission. Every verb above is a capability on the
calling principal — checked, audited, correlated — exactly like `bulk-submit` or
`extension-manage`:

| Capability | Grants | Profile |
| --- | --- | --- |
| `fleet-publish` / `fleet-serve` / `fleet-subscribe` | use the hackline message plane | edge + cloud |
| `ca-admin` | register / revoke / list devices at ca-server | cloud |
| `fleet-manage` | call hackline-gateway REST (list devices, send cmd, read events) | cloud |

## Edge and cloud profiles

The same rubix binary, the services placed by profile (SCOPE §"Edge and cloud
profiles"):

- **Edge (`role: edge`).** `hackline-agent` (a separate binary) bootstraps the device
  cert from ca-server and joins the fabric. rubix opens its shared `zenoh::Session`
  against that cert, constructs `Fleet`, publishes events / serves api / receives cmd,
  and ships data via `rubix-sync` — all rules still fire offline. ca-server and the
  gateway are remote; **neither is in rubix's offline boot path** (hackline already
  insulated cert-fetch behind systemd + renewal). If offline, no Zenoh session opens.
- **Cloud (`role: cloud`).** Runs the management side: `rubix-ca` against ca-server's
  admin API, and an HTTP client against `hackline-gateway`'s REST `/v1/*` + SSE
  (authed by a hackline **bearer token** — `owner`/`admin` with `device_scope` /
  `tunnel_scope`, per AUTH.md). This is where the UI's data comes from. ca-server +
  Vault and the gateway are co-located cloud services.

The cloud gateway-REST client is **not** allowed to link `hackline-gateway` (the
firewall above); it speaks the gateway's documented HTTP API like any other client.

## Auth seam — three layers, only the third is rubix's

A customer reaching `https://device-42.cloud.com/` through a hackline tunnel crosses
three independent authz layers. Conflating them is the classic fleet-auth tangle:

| Layer | Question | Owner | Mechanism (verified) |
| --- | --- | --- | --- |
| **L1 device access** | may user X reach device 42 at all? | hackline-gateway | bearer token `device_scope` (AUTH.md) |
| **L2 tunnel access** | may user X reach *this* tunnel? | hackline-gateway | bearer token `tunnel_scope` (AUTH.md) |
| **L3 in-device authz** | once on the device, may X read `/sidebar/...`? | rubix | `Gate` / capability grants |

L1+L2 already exist in hackline (scoped bearer tokens minted via
`POST /v1/users/:id/tokens`). L3 is rubix's existing model, unchanged. The **unsolved
seam** is carrying a gateway-verified *user identity* into a `role: edge` rubix whose
own auth is static-token/offline — so `Gate` can run L3 against a real user. The stale
INTEGRATION-RUBIX proposed a signed `X-Rubix-User` header (oauth2-proxy style); the
*shape* is reasonable but it must be redesigned against hackline's **current** bearer
model, not the stale doc. **This is Open Question 4 and gates any customer-facing
per-device URL.**

Three token regimes stay distinct, one per concern — do not unify them:

| Token | Between | Held by |
| --- | --- | --- |
| ca-server **HMAC preshared secret** | device/admin ↔ CA | hackline-agent (edge), `rubix-ca` (cloud admin) |
| **X.509 mTLS cert** | device ↔ Zenoh fabric | hackline-agent's renewal loop; rubix's session rides it |
| hackline **bearer token** (`device_scope`/`tunnel_scope`) | user ↔ gateway | rubix cloud `fleet-manage` client |

## Long-term: rubix UI to manage all of this

The end state the user asked for lands on the existing AdminX + nav pattern
(`AdminRules`, `AdminExtensions`; [PAGE-CONTEXT-AND-NAV.md](PAGE-CONTEXT-AND-NAV.md),
[DASHBOARDS-SCOPE.md](DASHBOARDS-SCOPE.md)). Two cloud-profile, RBAC-gated pages:

- **Fleet** — devices/agents with hackline session + tunnel health, open a device's
  URL (`https://device-N.cloud.com/`), tail its events (SSE), send operator commands.
  Backed by the `fleet-manage` gateway-REST client; live tail reuses the SSE deferral
  from EXTENSION-RUNTIME.
- **Identity / CA** — device certificate lifecycle: registered / issued / expiring /
  revoked, register a new device (mint its one-time secret), revoke, view/download the
  CA root. Backed by `rubix-ca`. Revocation here is the operator counterpart to the
  edge agent's auto-renewal.

The capabilities (`fleet-manage`, `ca-admin`) gate the buttons, not just the routes.
Browsing N devices is a device-switcher that opens per-device URLs — not an in-tree
unified fleet tree.

## Phasing

The identity→fabric leg is already shipped *by hackline*, so rubix work starts at the
message plane and the management UI — no edge-PKI phase to build.

1. **`rubix-fleet` event publisher (smallest seam).** Open the shared
   `zenoh::Session` against the agent-managed cert, wrap `ClientSession`, move
   slot-change events onto `publish_event` behind `fleet-publish`. Needs only
   hackline Phase 1.5 (already done). *Done test:* a slot-change event published on
   the edge appears at the gateway's events table / SSE.
2. **`rubix-fleet` api + cmd.** `serve_api` per route, `subscribe_cmd` with persistent
   dedupe (hackline Phase 2, done). *Done test:* a cloud-issued command installs/acks
   on the edge with an audit row.
3. **Cloud management clients.** `rubix-ca` admin + the `fleet-manage` gateway-REST
   client — the data layer the UI needs.
4. **UI pages.** Fleet + Identity/CA on AdminX, RBAC-gated.

Steps 2 onward must not expose customer-facing per-device URLs before the L1→L3 auth
seam (Open Question 4) is designed — an undecided seam gets accidentally fixed by
shipping.

## Open questions

1. **Whose cert does rubix's Zenoh session use?** Recommended: **reuse the cert
   `hackline-agent` already obtained** (point Zenoh's TLS at `/etc/hackline/certs/…`),
   so there is one device identity and one renewal loop. The alternative — rubix
   registers its own `global_uuid` and runs its own renewal — duplicates hackline's
   `[renewal]` logic and doubles the ca-server enrollment. Decide before Step 1; it
   determines where the shared `zenoh::Session` is constructed.
2. **One Zenoh session, confirmed?** This doc assumes `rubix-sync` and `rubix-fleet`
   share one `Arc<zenoh::Session>`. Confirm the lifecycle owner (likely `rubix-server`
   main) and that `rubix-sync`'s existing session construction can be hoisted to be
   shared, not forked.
3. **Identity reconciliation across id spaces.** rubix `agent_id`, the Zenoh ZID, and
   ca-server `global_uuid` (= cert CN) very likely name one device. Pick a canonical
   id and a mapping **before** Step 1 — three names for one device is a guaranteed
   field-mapping bug.
4. **The L1→L3 auth seam.** How a gateway-verified user identity reaches `role: edge`
   rubix for `Gate` to run L3 — redesigned against hackline's current bearer-token
   model. Gates customer-facing per-device URLs. Owned jointly with hackline.
5. **Where does ca-server run relative to rubix cloud?** It needs Vault, which rubix
   does not — leaning "external service rubix merely calls via `rubix-ca`", not
   something rubix supervises. Confirm before the Identity UI.
6. **Renewal under long offline windows.** A device offline past its cert expiry can't
   reach ca-server to renew (hackline's loop exits → systemd restarts → still no CA).
   Long Vault TTLs mitigate; the recovery path (re-enroll on contact) should be
   written down — this is hackline's failure mode but it surfaces in rubix's Fleet UI
   as "device unreachable", so the UI must distinguish *offline* from *cert-expired*.
7. **HMAC `admin` secret handling.** `rubix-ca` holds ca-server's `admin` preshared
   secret. Rotation = re-register; document the operator story for a leaked secret.

## Pointers

- hackline (authoritative): [`SCOPE.md`](../../../bin/hackline/SCOPE.md),
  [`DOCS/ARCHITECTURE.md`](../../../bin/hackline/DOCS/ARCHITECTURE.md),
  [`DOCS/AUTH.md`](../../../bin/hackline/DOCS/AUTH.md),
  [`DOCS/CERT-RENEWAL.md`](../../../bin/hackline/DOCS/CERT-RENEWAL.md),
  [`DOCS/CA-MANUAL.md`](../../../bin/hackline/DOCS/CA-MANUAL.md). The
  `bin/hackline/INTEGRATION-RUBIX.md` is **stale** — superseded by this document.
- ca-server: [`bin/ca-server/API.md`](../../../bin/ca-server/API.md),
  [`bin/ca-server/README.md`](../../../bin/ca-server/README.md).
- rubix data-plane sibling: [`rubix-sync`](../../crates/rubix-sync) and SCOPE
  §"Sync and conflict model".
- rubix principal/runtime model this reuses: [EXTENSION-RUNTIME.md](EXTENSION-RUNTIME.md),
  [SCOPE.md](../SCOPE.md) §5/§7. (CONTROL-ENGINE.md is referenced once only, for the
  trait-vs-concrete contrast — it is otherwise unrelated.)
