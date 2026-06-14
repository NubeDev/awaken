//! The rule model: definition, input bindings, and the composition registry.

mod bind;
mod define;
mod registry;

pub use bind::{Aggregate, Binding, resolve};
pub use define::Rule;
pub use registry::RuleRegistry;
