//! Extension-domain errors, converted into the project error at the boundary.
//!
//! An extension is a scoped service-account principal (`rubix/docs/sessions/
//! WS-13.md`); its failures are provisioning failures, capability-denied control
//! actions, and malformed JSON-RPC envelopes. They convert into the project
//! [`Error`](rubix_core::Error) so callers chain with `.context()` (CLAUDE.md
//! "Key Patterns"). A denied control action fails closed before any command is
//! applied — never a silent allow.

use rubix_core::Error as CoreError;

/// Convenience alias for extension results.
pub type Result<T> = std::result::Result<T, ExtError>;

/// Failures raised by the extension control/provisioning surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtError {
    /// Provisioning the extension's service-account principal failed.
    #[error("failed to provision extension: {0}")]
    Provision(String),

    /// Attaching a capability grant to the extension failed.
    #[error("failed to grant extension capability: {0}")]
    Grant(String),

    /// A JSON-RPC control action was refused before any command was applied —
    /// the extension lacked the capability the action requires (fail closed,
    /// `rubix/docs/sessions/WS-13.md`, contract #2).
    #[error("control action denied: {0}")]
    Denied(String),

    /// Routing a control action as a command through the WS-05 gate failed.
    #[error("control command failed: {0}")]
    Command(String),

    /// A JSON-RPC request envelope was malformed (bad method, missing params).
    #[error("malformed control request: {0}")]
    Request(String),

    /// Resolving the extension's scoped data-plane key-space failed.
    #[error("data-plane scope failed: {0}")]
    Scope(String),
}

impl From<ExtError> for CoreError {
    fn from(err: ExtError) -> Self {
        CoreError::Store(err.to_string())
    }
}
