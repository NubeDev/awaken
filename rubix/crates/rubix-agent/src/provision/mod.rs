//! Provision an AI agent as a scoped service-account principal at a tier.
//!
//! The agent leans on the **extensions-as-principals** model (AGENT.md open
//! question 6): it is provisioned on the same identity model as a user and any
//! extension — a [`Principal`](rubix_core::Principal) of kind `Extension`, scoped
//! to one namespace — and its cross-plane authority is a layered
//! [`AgentTier`] resolved onto the WS-04 grant set. The two authz layers stay
//! distinct: SurrealDB row-perms scope the data the agent reads, capability grants
//! scope the cross-plane actions it may take.

mod agent;
mod tier;

pub use agent::{AgentPrincipal, provision_agent};
pub use tier::AgentTier;
