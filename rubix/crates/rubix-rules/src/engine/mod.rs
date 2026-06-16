//! The embedded Rhai engine: setup and the decision a script produces.

mod decision;
mod register;

pub use decision::{Decision, from_dynamic};
pub use register::{build_engine, compile_check};
