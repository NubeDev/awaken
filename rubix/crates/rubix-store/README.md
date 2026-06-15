# rubix-store

The embedded SurrealDB store boundary for the rubix platform.

## What it provides

- **`StoreHandle`** — the durable read/write handle every persistence path goes through.
- **Connection & namespace bootstrap** — opening the embedded SurrealDB and selecting namespace/database.
- **Schema-init seam** — where collection/table schemas are applied.
- **`issue_scoped_session` / `ScopedSession`** — the seam that mints a scoped SurrealDB session so row-level permissions enforce read scope (used by `rubix-gate`).
- **Health probe** — liveness check for the store.

## Where it sits

The single owner of the SurrealDB connection. Everything durable goes through here; no other crate opens the database directly.

Authority: `rubix-store` row in `rubix/STACK-DEISGN.md` (contracts #1 and #6).
