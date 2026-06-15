//! The JSON-RPC control request envelope and its method set.
//!
//! The control plane is JSON-RPC (`rubix/docs/sessions/WS-13.md`): a request
//! names one [`ControlMethod`] and carries free-form JSON `params`. The method
//! set is closed — an unrecognised method is a malformed request, never a guessed
//! action (fail closed). Each mutating method maps to one
//! [`Capability`](rubix_gate::Capability) the extension must hold and one
//! [`Change`](rubix_gate::Change) routed through the WS-05 gate; the method-to-
//! capability mapping is the single point of choice, kept here so it cannot drift
//! across the verb files.

use rubix_core::{CorrelationId, Id};
use rubix_gate::Capability;

/// A method the extension control plane exposes over JSON-RPC.
///
/// The set is closed (`rubix/docs/sessions/WS-13.md`, SCOPE): `register`,
/// `configure`, `invoke`, `health`, and `lifecycle`. The stable wire form is the
/// kebab/lower string from [`ControlMethod::as_str`]; an unknown string resolves
/// to `None` so the dispatcher fails closed rather than guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMethod {
    /// Register the extension's configuration record.
    Register,
    /// Update the extension's configuration record.
    Configure,
    /// Invoke an extension action (the gated, audited effect).
    Invoke,
    /// Probe the extension's liveness (read-only, no command).
    Health,
    /// Start / stop / disable the extension's lifecycle state.
    Lifecycle,
}

impl ControlMethod {
    /// Every control method, in declaration order.
    pub const ALL: [ControlMethod; 5] = [
        ControlMethod::Register,
        ControlMethod::Configure,
        ControlMethod::Invoke,
        ControlMethod::Health,
        ControlMethod::Lifecycle,
    ];

    /// The stable wire string for this method.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ControlMethod::Register => "register",
            ControlMethod::Configure => "configure",
            ControlMethod::Invoke => "invoke",
            ControlMethod::Health => "health",
            ControlMethod::Lifecycle => "lifecycle",
        }
    }

    /// Resolve a wire string to a known method, or `None` to fail closed.
    #[must_use]
    pub fn parse(raw: &str) -> Option<ControlMethod> {
        ControlMethod::ALL
            .into_iter()
            .find(|method| method.as_str() == raw)
    }

    /// The capability this method requires, if it is a gated command.
    ///
    /// [`Health`](ControlMethod::Health) is read-only and returns `None` — it
    /// crosses no command and writes no audit row. Every other method is a
    /// command and names the WS-04 capability the extension must hold. The
    /// mapping lives here so the gate check and the routed command cannot use
    /// different capabilities.
    #[must_use]
    pub fn required_capability(self) -> Option<Capability> {
        match self {
            ControlMethod::Register | ControlMethod::Configure => {
                Some(Capability::DatasourceRegister)
            }
            ControlMethod::Invoke => Some(Capability::RuleInvoke),
            ControlMethod::Lifecycle => Some(Capability::DatasourceRegister),
            ControlMethod::Health => None,
        }
    }
}

/// A JSON-RPC control request: a method plus its free-form params.
///
/// The `target` is the extension's own control record id the method acts on
/// (configuration / lifecycle state), addressed in the extension's namespace; the
/// gate has no cross-tenant write path, so the effect always lands in the calling
/// principal's namespace.
#[derive(Debug, Clone)]
pub struct ControlRequest {
    /// The method to dispatch.
    pub method: ControlMethod,
    /// The control record the method acts on.
    pub target: Id,
    /// The free-form JSON parameters the method consumes.
    pub params: serde_json::Value,
}

impl ControlRequest {
    /// Build a control request for `method` acting on `target` with `params`.
    #[must_use]
    pub fn new(method: ControlMethod, target: Id, params: serde_json::Value) -> Self {
        Self {
            method,
            target,
            params,
        }
    }
}

/// The outcome of a mutating control method routed through the gate.
///
/// Carries the correlation id the gate stamped onto the command and its audit
/// row — the thread a caller follows into the audit and trace planes
/// (`rubix/docs/sessions/WS-13.md`, contract #1). A read-only method
/// ([`health`](super::health)) returns its own status type instead.
#[derive(Debug, Clone)]
pub struct ControlOutcome {
    /// The control record the method acted on.
    pub target: Id,
    /// The correlation id the gate carried onto the command and audit row.
    pub correlation_id: CorrelationId,
}
