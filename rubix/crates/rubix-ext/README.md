# rubix-ext

Extensions as scoped principals for the rubix platform.

## What it provides

An extension is modelled as a **service account on the same identity model as a user** — a scoped `rubix_core::Principal` of kind `Extension` bound to one namespace, not a privileged plugin-trust path. Three faces:

- **`provision`** — `register_extension` registers the scoped service-account principal; `grant_extension` attaches its capability grants. The same grant mechanism expresses a read-only, ingest-only, or admin extension — only the `GrantProfile` differs.
- **`control`** — the JSON-RPC control plane: `register` / `configure` / `invoke` / `health` (`probe_health`) / `lifecycle`. Every mutating method crosses the gate as a `rubix_gate::Command`, so it is capability-checked, correlated, and audited identically to a user's; an out-of-grant call is denied before any effect.
- **`data`** — `authorize_data_scope` delegates the data plane to Zenoh key-space scoping (scope resolved once at subscribe).

## Where it sits

Enforcement reuses the two layers users already have: SurrealDB row-level permissions scope readable data, and app-enforced capability grants scope cross-plane actions. No new trust path.

Authority: `rubix/docs/SCOPE.md` (principle 5, "Extensions as principals"); `rubix/docs/sessions/WS-13.md` (contracts #1, #2).
