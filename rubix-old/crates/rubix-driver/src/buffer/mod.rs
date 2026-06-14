//! Outbound data-plane buffering with declared per-channel delivery semantics,
//! per STACK-DEISGN.md "Ack/backpressure" and `docs/sessions/WS-10.md`:
//!
//! - `cur` is latest-wins: a bounded [`CurBuffer`] drops the oldest sample under
//!   pressure and counts every drop (never silent truncation).
//! - `write`/`his` are reliable: a bounded [`ReliableQueue`] backpressures with
//!   [`crate::DriverError::BufferFull`] instead of dropping, and `write` adds an
//!   ack/retry protocol ([`PendingWrite`]/[`WriteAck`]) that gives up with
//!   [`crate::DriverError::AckTimeout`] rather than retrying forever.
//!
//! Pure logic only — ordering, bounds, counters, and retry policy. The zenoh IO
//! that drains these buffers lives in the driver (`rubix-driver-sim`).

mod ack;
mod cur;
mod policy;
mod reliable;

pub use ack::{PendingWrite, WriteAck, WriteAction};
pub use cur::CurBuffer;
pub use policy::{ChannelClass, OverflowPolicy};
pub use reliable::ReliableQueue;
