//! The extension's data plane — delegated to WS-12 Zenoh key-space scoping.
//!
//! The data plane is **not re-implemented here** (`rubix/docs/sessions/
//! WS-13.md`, SCOPE): an extension subscribes to the streaming fabric through the
//! *same* WS-12 path a user does. Scope is resolved once at subscribe — a single
//! capability decision (the [`ZenohSubscribe`](rubix_gate::Capability::ZenohSubscribe)
//! grant plus the edge-partition check), never re-taxed per message (contract
//! #2). [`scope`] is the thin delegation seam: it forwards to
//! [`authorize_keyspace`](rubix_ingest::authorize_keyspace) so the extension's
//! data scope is decided by the one ingest authority, with no extension-only
//! scoping logic that could drift from it.

mod scope;

pub use scope::authorize_data_scope;
