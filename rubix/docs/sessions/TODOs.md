# Rubix Build — Blocker / Follow-up Log

Append-only log of blockers and deferred follow-ups raised by unattended workstream sessions. A
session that cannot proceed properly (genuine ambiguity, a missing dependency a not-yet-run WS owns,
or a contract exception that would otherwise require a hack) logs here, sets its STATUS.md row to ⛔
blocked, commits what compiles, and stops.

Format (one entry per blocker):

```
## <UTC timestamp> — WS-xx — <one-line title>
- **What:** what is blocked and where (file:line if known).
- **Why:** why it can't be done properly now (the dependency / ambiguity / contract conflict).
- **Needs:** what unblocks it (which WS, which human decision).
- **Resolution:** (filled when resolved — struck through or dated. The loop resets the row to ⬜
  once this is resolved.)
```

When the human resolves a blocker, strike the entry through or add a `Resolution:` line; the loop
then resets that WS's row to ⬜ and re-picks it in queue order.

---

## 2026-06-15T06:15:00Z — WS-13/14/15 — Deferred (edge/extensions/sync infrastructure)

- **What:** WS-13 (extensions), WS-14 (edge/cloud profiles), WS-15 (sync shipper) marked blocked.
- **Why:** User focus is on core backend transport (WS-16) first; edge/extension/sync infrastructure deferred.
- **Needs:** User decision when to implement. For now, WS-12 → WS-16 directly (skip 13/14/15).
- **Resolution:** Unblock when user is ready to ship edge/extensions features.

## 2026-06-15T09:50:00Z — WS-16 — Transport sub-deliverables that require deferred WSs

- **What:** Three WS-16 sub-deliverables stand on deferred crates and were NOT
  implemented (the rest of WS-16 — HTTP CRUD via the gate, scoped reads, query,
  datasources list, WS live-query bridge, OpenAPI, and `rubix-prefs` — shipped):
  1. **JSON-RPC extension control endpoint** (`rpc/control.rs` → WS-13). The
     control plane dispatches register/configure/invoke/health to an extension
     principal; `rubix-ext` (WS-13) owns that contract and does not exist yet.
  2. **Datasource registration route** (`POST /datasources`). `rubix_datasource::
     register` needs a `Connector` instance to materialise; connector instances
     are supplied by the WS-13 extension model. `GET /datasources` (list) shipped.
  3. **Profile selection into `AppState`** (WS-14). `main.rs` boots on the
     committed `RuntimeConfig` edge default; edge/cloud profile *selection* (cargo
     feature + `RUBIX_PROFILE`) is WS-14's contract. The transport is profile-
     agnostic and works on the default edge profile today.
- **Why:** Implementing these now would require redefining the WS-13/WS-14
  contracts in `rubix-server`, which violates the stay-in-lane rule. Stubbing the
  JSON-RPC endpoint or a registration route would be a placeholder that pretends
  to work, forbidden by CLAUDE.md.
- **Needs:** WS-13 (`rubix-ext`) for the JSON-RPC control plane and datasource
  registration via a connector; WS-14 for profile selection.
- **Note:** Record mutations are routed through the gate's `IngestPublish`
  capability (the committed enum has no dedicated record-write variant). If the
  gate's `Capability` enum later gains a record-write capability, the one place to
  change is `crates/rubix-server/src/http/records/capability.rs`.
- **Resolution:** Wire `rpc/control.rs` + `POST /datasources` when WS-13 lands, and
  profile selection when WS-14 lands. (3) Profile selection into `AppState`
  resolved 2026-06-15T12:00:00Z — `rubix-server` `profile` module selects from
  `RUBIX_PROFILE` and threads the `Profile` into `AppState::with_profile` in
  `main.rs`; the edge/cloud cargo features and fail-closed boot landed with WS-14.
