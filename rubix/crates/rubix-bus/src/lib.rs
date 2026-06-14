//! Event bus for the rubix platform — the eventing spine.
//!
//! Two of the three planes from `rubix/docs/SCOPE.md` ("Event bus"), presented
//! as one eventing surface:
//!
//! - **In-process** ([`inprocess`]) — tokio broadcast channels for
//!   component-to-component control events inside the binary, no serialization,
//!   no network.
//! - **Data-change** ([`livequery`]) — SurrealDB live queries as the pub/sub for
//!   "a record appeared/changed". A subscription opens on a gate-issued **scoped
//!   session** (`rubix-gate`), so SurrealDB row-level permissions decide which
//!   records a principal sees — the scope is set once at subscribe, not proxied
//!   per message (contract #1, `rubix/STACK-DEISGN.md`).
//!
//! The Zenoh stream/transport plane is WS-12, not this crate.

mod error;
mod event;
pub mod inprocess;
pub mod livequery;

pub use error::{BusError, Result};
pub use event::ControlEvent;
pub use inprocess::{ControlBus, ControlSubscription, publish, subscribe};
pub use livequery::{DataChange, DataChangeKind, DataChangeStream, subscribe_table};
