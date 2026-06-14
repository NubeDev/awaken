//! Rubix server library: `AppState` wiring and the HTTP router.
//!
//! The binary (`main.rs`) opens the store and serves [`router`]; this library
//! exists so the router can be exercised in integration tests without binding a
//! socket. Crate role: `rubix/STACK-DEISGN.md` (`rubix-server` row).

mod health;
mod state;

pub use state::{AppState, router};
