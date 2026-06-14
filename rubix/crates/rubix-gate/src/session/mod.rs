//! Scoped read sessions: gate-issued, principal-bound SurrealDB sessions.
//!
//! A scoped session is minted once per principal (contract #1) and confines
//! reads to the principal's namespace via SurrealDB row-level permissions
//! (contract #2). `scope` binds the session by signing in; `issue` is the public
//! verb that produces the [`ScopedSession`].

mod issue;
mod scope;

pub use issue::{ScopedSession, issue_scoped_session};
