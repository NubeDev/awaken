//! The native SQL text and its bound parameters — the two halves the executor
//! keeps separate so values are never spliced into SQL.

mod bind;
mod single;

pub use bind::{Param, Params};
pub use single::ensure_single_statement;
