//! The stored-rule model and the abstract store the resolver loads from.
//!
//! A [`StoredRule`] is the in-memory representation of a saved rule (id, name,
//! script, and a parameter schema). [`RuleStore`] abstracts *where* rules are
//! loaded from so this crate is testable without a database — the integrating
//! session provides the real store backed by a rules table. The resolver
//! ([`crate::compose`]) loads a rule by name for composition.
//!
//! The design distinguishes stored *functions* (return a value/frame) from
//! stored *rules* (return a verdict). This crate ships the rule half — the emit
//! unit a spark composes — and the [`RuleStore`] trait is shaped so a function
//! store can be added alongside it without changing the rule path.

mod load;
mod memory;
mod record;

pub use load::RuleStore;
pub use memory::MemoryRuleStore;
pub use record::{ParamSchema, ParamSpec, StoredRule};
