//! Run-path fixtures: evaluate an inline script over a frame with a store.

use std::sync::Arc;

use rhai::Map;
use rubix_rules::{
    run_rule, Frame, MemoryRuleStore, RuleError, RuleResult, RuleSource, SandboxLimits,
};

/// Evaluate `script` over `frame` with the given store and default limits.
#[allow(dead_code)]
pub fn run_with(
    store: MemoryRuleStore,
    script: &str,
    frame: Frame,
) -> Result<RuleResult, RuleError> {
    run_rule(
        Arc::new(store),
        RuleSource::Inline(script),
        frame,
        Map::new(),
        SandboxLimits::default(),
    )
}

/// Evaluate `script` over `frame` with an empty store and `limits`.
#[allow(dead_code)]
pub fn run_limited(
    script: &str,
    frame: Frame,
    limits: SandboxLimits,
) -> Result<RuleResult, RuleError> {
    run_rule(
        Arc::new(MemoryRuleStore::new()),
        RuleSource::Inline(script),
        frame,
        Map::new(),
        limits,
    )
}
