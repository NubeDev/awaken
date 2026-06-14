//! Test fixture: an in-memory store, a scoped session, and grant helpers.
//!
//! The datasource registry's authorize/span verbs read real SurrealDB auth and
//! capability grants on kv-mem (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem
//! for tests). This fixture opens the durable handle with the gate + audit schema,
//! provisions a principal, issues its scoped session, and grants capabilities, so
//! each test exercises the genuine fail-closed path rather than a mock.

pub mod open;
