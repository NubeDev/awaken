//! Edge↔cloud sync shipper for the rubix platform.
//!
//! SurrealDB has no mature multi-master replication, so sync is an
//! **application-level shipper over Zenoh**, not DB replication, with an explicit
//! conflict model (`rubix/docs/SCOPE.md`, "Sync and conflict model";
//! `rubix/STACK-DEISGN.md`, `rubix-sync` row). The two planes have deliberately
//! different conflict shapes:
//!
//! - **Data plane** ([`data`]) — readings, insights, audit, traces — is
//!   append-only and edge-owned, partitioned by edge identity (contract #5). Two
//!   edges never mint the same id, so reconciliation at the cloud is **ordering +
//!   dedup by id, not merge**: there is no multi-master conflict by construction.
//!   The shipper moves records over Zenoh, re-sends unacked records on reconnect,
//!   and dedups re-sends so a replay is an idempotent no-op.
//! - **Config plane** ([`config`]) — dashboards, rules, tags, datasource defs —
//!   is the only surface that ever reconciles. **Ownership** is the first rule
//!   (cloud owns shared/tenant config, edge owns local-only); only an unavoidable
//!   overlap falls back to **last-write-wins**, with the WS-05 audit timestamp as
//!   the tiebreak. No CRDT (deferred per the SCOPE open question).
//!
//! The crate is laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`).

mod error;

pub mod config;
pub mod data;

pub use config::{
    ConfigScope, ConfigVersion, LwwOutcome, Owner, break_tie, last_write_wins, owner_of, reconcile,
    reconcile_ambiguous,
};
pub use data::{
    Outbox, SYNC_DATA_ROOT, SYNC_READING_ROOT, SeenSet, apply_batch, apply_reading,
    apply_reading_batch, apply_record, decode_reading, decode_record, encode_reading,
    encode_record, in_apply_order, publish_reading, publish_record, reading_ship_key, ship_key,
};
pub use error::{Result, SyncError};
