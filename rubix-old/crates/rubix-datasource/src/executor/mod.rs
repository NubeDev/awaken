//! The single validation+caps execution path and its two entry points
//! (raw operator SQL, named-query invocation).

mod cap;
mod run;

pub use run::Executor;
