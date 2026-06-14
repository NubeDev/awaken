//! Build a hardened Rhai engine for one execution.

use rhai::module_resolvers::DummyModuleResolver;
use rhai::{Dynamic, Engine};

use super::deadline::Deadline;
use super::limits::SandboxLimits;

/// Construct a sandboxed [`Engine`] enforcing `limits` against `deadline`.
///
/// The returned engine has no curated surface registered yet — that is the
/// caller's job ([`crate::register`]) so the sandbox stays orthogonal to the API
/// it hosts. What the engine *does* have is every escape hatch closed:
///
/// - operation, call-level, and string/array size caps from `limits`;
/// - a wall-clock cut-off: the `on_progress` callback returns `Some` once the
///   shared `deadline` passes, which Rhai turns into a termination error the
///   run loop maps to [`RuleError::LimitExceeded`];
/// - imports disabled *explicitly* via [`DummyModuleResolver`]. This is the
///   documented Rhai footgun the design calls out: not registering a module is
///   not the same as blocking `import`, so the resolver is installed to fail
///   every `import` closed. (Covered by a test.)
///
/// No file, network, or `eval` APIs are registered — `Engine::new` ships none of
/// those, and nothing here adds them.
///
/// [`RuleError::LimitExceeded`]: crate::RuleError::LimitExceeded
pub fn build_engine(limits: &SandboxLimits, deadline: Deadline) -> Engine {
    let mut engine = Engine::new();

    engine.set_max_operations(limits.max_operations);
    engine.set_max_call_levels(limits.max_call_levels);
    engine.set_max_string_size(limits.max_string_size);
    engine.set_max_array_size(limits.max_array_size);

    // Imports must be blocked explicitly; absence of registration is not enough.
    engine.set_module_resolver(DummyModuleResolver::new());

    // Wall-clock cut-off. Returning `Some` aborts the run; the value is unused.
    engine.on_progress(move |_ops| {
        if deadline.expired() {
            Some(Dynamic::UNIT)
        } else {
            None
        }
    });

    engine
}
