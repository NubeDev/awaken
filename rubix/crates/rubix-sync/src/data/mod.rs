//! The data-plane shipper: append-only records edge→cloud, conflict-free.
//!
//! The data plane (readings, insights, audit, traces) is append-only and
//! edge-owned, partitioned by edge identity (`rubix/STACK-DEISGN.md`, contract
//! #5). Two edges never mint the same id, so reconciliation at the cloud is
//! **ordering + dedup by id, not merge** — there is no multi-master conflict by
//! construction. The shipper moves records over Zenoh ([`ship`]); the receiver
//! applies them in [`order`] and drops re-sends with [`dedup`]; the edge re-sends
//! unacked records on reconnect through [`replay`], which combined with dedup is
//! idempotent (a re-sent record is a no-op).

pub mod dedup;
pub mod order;
pub mod replay;
pub mod ship;
pub mod ship_reading;

pub use dedup::SeenSet;
pub use order::in_apply_order;
pub use replay::Outbox;
pub use ship::{
    SYNC_DATA_ROOT, apply_batch, apply_record, decode_record, encode_record, publish_record,
    ship_key,
};
pub use ship_reading::{
    SYNC_READING_ROOT, apply_reading, apply_reading_batch, decode_reading, encode_reading,
    publish_reading, reading_ship_key,
};
