# rubix-server

The transport that wires every committed crate to the wire — the integration layer.

## What it provides

- **`router(state)`** — assembles the full transport router: HTTP routes, the WebSocket live-query bridge, and the OpenAPI document route, over a shared `AppState`.
- **HTTP** — axum routes. Mutations go through the WS-05 gate as commands; reads run on the WS-03 scoped session. Health, records CRUD, collection listing.
- **WebSocket** (`ws`) — a bridge over the in-process live-query feed.
- **OpenAPI** (`openapi_document`) — the utoipa OpenAPI 3.1 document, served and exported.
- **Profiles** (`profile`) — edge vs cloud boot, tenant resolution, backend verification; fail-closed selection (`Profile`, `NamespaceStrategy`).
- **Datasources** (`datasources`) — schema definition and rehydration of registered datasources.
- **`rubix-prefs` layer** — applies per-user display preferences at the DTO layer.
- **`seed_dev`** — development seed data.

## Where it sits

The top of the stack and the only crate that binds a socket. The library assembles `router` so integration tests exercise routes without a socket; `main.rs` opens the store, builds `AppState`, and serves.

Authority: `rubix-server` row in `rubix/STACK-DEISGN.md`; `rubix/docs/sessions/WS-16.md`.
