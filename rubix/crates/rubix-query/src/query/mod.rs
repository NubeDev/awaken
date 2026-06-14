//! The unified read-only query surface.
//!
//! Accepts a single `SELECT`/`WITH` statement over the canonical tables, scans
//! those tables through the principal's scoped session, and runs the SQL in
//! DataFusion (`rubix/STACK-DEISGN.md`, contract #6: DataFusion above SurrealDB
//! only for unification/aggregation). The query action is app-enforced — gated by
//! a WS-04 capability before any scan runs (contract #2).

mod authorize;
mod guard;
mod run;

pub use authorize::run_authorized;
pub use guard::ensure_read_only;
pub use run::run;
