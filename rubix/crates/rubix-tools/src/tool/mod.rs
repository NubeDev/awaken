//! Rubix BMS tools exposed to awaken agents. One verb per file; each is a
//! [`awaken_runtime_contract::contract::tool::TypedTool`] over the BMS access
//! port, so the runtime gets a schema and arg validation for free.

mod query;
mod read_point;
mod write_point;

pub use query::{QueryArgs, QueryTool};
pub use read_point::{ReadPointArgs, ReadPointTool};
pub use write_point::{WritePointArgs, WritePointTool};
