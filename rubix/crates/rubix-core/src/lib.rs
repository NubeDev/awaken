//! Domain model and shared contracts for the rubix platform.
//!
//! Scope authority: `rubix/docs/SCOPE.md`. Crate role and contracts:
//! `rubix/STACK-DEISGN.md` (`rubix-core` row + load-bearing contracts #3, #6).

mod configure;
mod correlate;
mod error;
mod id;

pub use configure::{Profile, RuntimeConfig, StoreEngine};
pub use correlate::CorrelationId;
pub use error::{Error, Result, ResultExt};
pub use id::Id;
