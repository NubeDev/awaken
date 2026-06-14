//! Zenoh data plane: publishes live `cur` values, subscribes to driver `cur`
//! publications, and serves `write`/`his` queryables against the store, per the
//! keyexpr scheme in STACK-DEISGN.md. This module is the only transport-aware
//! part of the server; the store and HTTP layers stay zenoh-free.

mod open;
mod publish_cur;
mod publish_spark;
mod serve;
mod subscribe_cur;

use std::sync::Arc;

use zenoh::Session;

use crate::store::Store;

/// Handle to the zenoh session and its declared queryables. Cheap to clone;
/// the session is shared. Held in [`crate::AppState`] so HTTP handlers can
/// publish `cur` after a store mutation.
#[derive(Clone)]
pub struct ZenohBus {
    session: Arc<Session>,
    store: Store,
}
