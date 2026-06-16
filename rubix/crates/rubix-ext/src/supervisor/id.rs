//! [`ExtensionId`] — the supervisor's per-extension key.
//!
//! The starter supervisor keyed its handle map by a manifest-derived
//! `ExtensionId` string. rubix has no manifest: an extension *is* a scoped
//! [`Principal`](rubix_core::Principal) (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "What rubix means by an extension that runs"). So the runtime's key is the
//! principal's identity — its `subject` within its `namespace` — not a separate
//! id space. Keying by both means a supervisor map is **per-tenant by
//! construction**: two namespaces can never collide on a bare subject, matching
//! the per-namespace scoping every admin endpoint enforces.

use rubix_core::Principal;

/// The identity the supervisor registry keys an extension's handle by.
///
/// Derived from the extension [`Principal`] — `(namespace, subject)` — so the
/// runtime never invents an id space parallel to the gate's. Cheap to clone and
/// hash; used as the `HashMap` key in
/// [`SupervisorRegistry`](crate::supervisor::SupervisorRegistry) and the
/// [`MetricsRegistry`](crate::metrics::MetricsRegistry).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtensionId {
    /// The namespace the extension principal is scoped to.
    namespace: String,
    /// The extension principal's subject.
    subject: String,
}

impl ExtensionId {
    /// Build an id from a raw namespace/subject pair (e.g. decoded from a
    /// lifecycle record at boot).
    #[must_use]
    pub fn new(namespace: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            subject: subject.into(),
        }
    }

    /// The extension's namespace.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The extension principal's subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl From<&Principal> for ExtensionId {
    fn from(principal: &Principal) -> Self {
        Self {
            namespace: principal.namespace.clone(),
            subject: principal.subject.as_str().to_owned(),
        }
    }
}

impl std::fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.namespace, self.subject)
    }
}
