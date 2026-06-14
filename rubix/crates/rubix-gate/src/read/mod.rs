//! Scoped reads: SELECTs that run on a principal's session.
//!
//! The read enforcement point (contract #1): reads run on the gate-issued scoped
//! session and are filtered by SurrealDB row-level permissions, never proxied
//! per message.

mod on_session;

pub use on_session::{read_record_on_session, read_records_on_session};
