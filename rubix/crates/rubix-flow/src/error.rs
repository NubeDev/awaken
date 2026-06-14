//! Board loading/build errors and the typed seam error.

/// Board loading/build errors.
#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("unknown board component `{0}`")]
    UnknownComponent(String),
    #[error("board build failed: {0}")]
    Build(String),
}

/// Why a [`crate::PointAccess`] call failed. The seam is stable and host-facing,
/// so it carries error *categories* rather than an opaque `anyhow` chain: a node
/// (and the host) can tell a fail-closed capability gap from a tenancy denial
/// from a backing-store failure, instead of string-matching a message.
///
/// `NotFound` is intentionally absent until the store layer types its own
/// lookup errors — today a missing keyexpr surfaces as `Store`, and a point with
/// no current value is `Ok(None)`, not an error.
#[derive(Debug, thiserror::Error)]
pub enum FlowAccessError {
    /// This access does not back the requested capability (the fail-closed
    /// default for `emit_spark`/`request_agent`/`query_datasource` on an access
    /// wired without that backend — e.g. the agent's own board access).
    #[error("{0}")]
    Unsupported(String),
    /// The call names a resource outside the run's authorized tenant scope.
    #[error("{0}")]
    Denied(String),
    /// The backing store or service failed while serving the call.
    #[error("{0}")]
    Store(String),
}

impl FlowAccessError {
    /// Construct a [`FlowAccessError::Store`] from any error's display form. Used
    /// at the boundary where a host impl folds its backend error into the seam.
    pub fn store(e: impl std::fmt::Display) -> Self {
        FlowAccessError::Store(e.to_string())
    }
}
