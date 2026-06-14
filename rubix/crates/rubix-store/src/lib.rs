//! Embedded SurrealDB store boundary for the rubix platform.
//!
//! Owns connection/namespace bootstrap, a schema-init seam, the durable
//! read/write handle, a health probe, and the scoped-session issuance seam.
//! Crate role and contracts: `rubix/STACK-DEISGN.md` (`rubix-store` row,
//! contracts #1 and #6).

mod bootstrap;
mod check_health;
mod connect;
mod error;
mod handle;
mod init_schema;
mod issue_session;

pub use error::{Result, StoreError};
pub use handle::StoreHandle;
pub use issue_session::{ScopedSession, issue_scoped_session};
