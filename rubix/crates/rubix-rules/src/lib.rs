//! Composable spark rules: sandboxed Rhai scripts over a vectorized engine.
//!
//! A *rule* is a Rhai script that turns caller-supplied rows (an Arrow
//! `RecordBatch`) into a decision — `flagged` / `severity` / `message` — by
//! composing curated, vectorized primitives. The script orchestrates; the engine
//! computes. User code never loops over rows (the non-negotiable design rule):
//! it chains DataFusion-backed primitives on a [`Frame`] handle and writes the
//! threshold / naming / decision logic around them.
//!
//! This crate is the engine, the curated primitive surface, the rule-result and
//! `finding` constructor, an abstract [`RuleStore`] with an in-memory
//! implementation, and the `rule(name, frame, params)` composition primitive
//! with cycle/depth bounding. It is **standalone** — it depends on no other
//! rubix crate and queries no database; the caller hands in the frame, and an
//! integrating session wires this into the board rule node, a real store, and
//! the rubix-core severity mapping.
//!
//! # Entry point
//!
//! [`run_rule`] evaluates an inline script or a stored rule over a [`Frame`] and
//! returns a [`RuleResult`] — without emitting a finding (the same path serves
//! the design's dry-run). A non-flagged result is the normal "ran, found
//! nothing" outcome, returned as `Ok`; only a broken rule or a composition
//! failure is a [`RuleError`].

mod compose;
mod error;
mod frame;
mod register;
mod result;
mod run;
mod sandbox;
mod severity;
mod store;

pub use error::RuleError;
pub use frame::Frame;
pub use result::RuleResult;
pub use run::{run_rule, RuleSource};
pub use sandbox::SandboxLimits;
pub use severity::Severity;
pub use store::{MemoryRuleStore, ParamSchema, ParamSpec, RuleStore, StoredRule};

// Re-exported for callers that build a frame from Arrow data they already hold.
pub use datafusion::arrow::datatypes::SchemaRef;
pub use datafusion::arrow::record_batch::RecordBatch;
