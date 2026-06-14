//! SurrealDB-native record permissions (contract #2).
//!
//! Defines the principal access method and the row-level read permission that
//! confines a scoped session to its principal's namespace. The engine enforces
//! it; the app does not filter (`rubix/STACK-DEISGN.md`, contract #2).

mod define;

pub use define::define_gate_schema;
