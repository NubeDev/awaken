//! Register the curated surface into a sandboxed engine.
//!
//! Three groups are registered, all operating on values a script may hold — and
//! nothing else (no file/network/eval, no row iteration):
//!
//! - [`frame_api`] — the [`Frame`](crate::Frame) custom type and its primitives
//!   (`select`, `rolling_mean`, `resample`, `zscore`, `anomalies`, …). Scripts
//!   chain these; the engine computes.
//! - [`result_api`] — the `finding(severity, message)` constructor and the
//!   `RuleResult` accessors, the rule's return type.
//! - [`compose_api`] — the `rule(name, frame, params)` composition primitive,
//!   bounded by the shared budget and the cycle/depth guard.
//!
//! A primitive that fails records a typed [`RuleError`](crate::RuleError) on the
//! execution's error sink and throws a sentinel; the run loop recovers the
//! category, so a `resolve` error from composition is never flattened into a
//! generic runtime error by Rhai's string error channel.

mod compose_api;
mod frame_api;
mod result_api;

use std::sync::Arc;

use rhai::{Engine, EvalAltResult};

use crate::error::RuleError;
use crate::run::Execution;

pub(crate) use compose_api::register_compose;
pub(crate) use frame_api::register_frame;
pub(crate) use result_api::register_result;

/// Register the whole curated surface into `engine` for `exec`.
pub(crate) fn register_all(engine: &mut Engine, exec: Arc<Execution>) {
    register_frame(engine, exec.clone());
    register_result(engine);
    register_compose(engine, exec);
}

/// Bridge a `Result<T, RuleError>` from a primitive into a Rhai result.
///
/// On error the typed category is stashed on the execution sink (so the run loop
/// can recover compile/runtime/limit/resolve) and a sentinel `EvalAltResult` is
/// raised to unwind the script.
pub(crate) fn bridge<T>(
    exec: &Execution,
    result: Result<T, RuleError>,
) -> Result<T, Box<EvalAltResult>> {
    result.map_err(|err| {
        let message = err.to_string();
        exec.set_error(err);
        message.into()
    })
}
