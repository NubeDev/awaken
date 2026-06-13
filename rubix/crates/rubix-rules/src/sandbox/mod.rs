//! The Rhai sandbox: a hardened `Engine` factory for running authored scripts.
//!
//! Rules run on a scheduled cadence over live tenant data, so the engine is
//! locked down before any script touches it. Per the design doc and RW-06's
//! `sandbox.rs`, this enforces operation / call-level / size limits, a
//! wall-clock deadline, and — the documented Rhai footgun — *explicitly*
//! disabled imports: absence of a registered module is not enough to block
//! `import`, so a [`DummyModuleResolver`] is installed.
//!
//! One engine is built per execution (cheap) so there is no cross-tenant state.
//!
//! [`DummyModuleResolver`]: rhai::module_resolvers::DummyModuleResolver

mod build;
mod deadline;
mod limits;

pub use build::build_engine;
pub use deadline::Deadline;
pub use limits::SandboxLimits;
