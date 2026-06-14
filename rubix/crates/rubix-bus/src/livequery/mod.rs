//! Data-change plane: SurrealDB live queries as record-change pub/sub.
//!
//! "A record appeared/changed" pushed to a subscriber over a gate-issued scoped
//! session, so SurrealDB row-level permissions decide which records the
//! principal sees — scope set once at subscribe, not per message (contract #1,
//! `rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Event bus"). [`subscribe_table`]
//! opens the subscription; [`DataChangeStream`] yields a [`DataChange`] per
//! visible record change.

mod change;
mod stream;
mod subscribe;

pub use change::{DataChange, DataChangeKind};
pub use stream::DataChangeStream;
pub use subscribe::subscribe_table;
