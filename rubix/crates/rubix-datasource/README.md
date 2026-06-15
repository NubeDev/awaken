# rubix-datasource

Pluggable datasource connectors for the rubix query surface — the Grafana-style datasource model.

## What it provides

- **`Connector` trait** — declared config in, a DataFusion `TableProvider` out. "Unlimited datasources" means adding a `Connector` impl, not changing the core.
- **`Registry`** — keyed by datasource id, seeded with the native SurrealDB default (`NATIVE_SURREAL_ID`), that the spanning query reads from. `register`, `register_materialized`, `find`, `list`, `remove`, `resolve`.
- **Authorization** — `register` is gated by the `datasource-register` capability, `span` by `external-query`; both fail closed (`authorize_register`).
- **`SurrealConnector`** — the native connector. **`PostgresConnector`** — feature-gated (`#[cfg(feature = "postgres")]`), absent on the default edge build (so it fails closed there).

## Where it sits

The layer above `rubix-query`'s unified surface — it supplies the additional `TableProvider`s the spanning query unifies over.

Authority: `rubix/docs/SCOPE.md` ("Datasources"); contract #2 in `rubix/STACK-DEISGN.md`.
