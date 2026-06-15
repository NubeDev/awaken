//! Resolve the namespace a request's scoped session signs into, per profile.
//!
//! This is where the profile's namespace strategy meets the gate's tenant
//! resolution (WS-03). On an edge profile there is no multi-tenancy code path: the
//! one configured namespace is authoritative and any tenant hint is ignored
//! (`rubix/docs/SCOPE.md`, "No multi-tenancy on edge"). On a cloud profile the
//! per-tenant namespace is derived from the request's tenant, falling back to the
//! configured namespace only when no tenant is carried (the control surface that
//! has no tenant context). Auth then signs the scoped session into the resolved
//! namespace, so a tenant only ever sees its own rows.

use super::ProfileError;
use super::define::{NamespaceStrategy, Profile};

impl Profile {
    /// The namespace a request resolves to under this profile.
    ///
    /// `configured` is the boot-time namespace (`AppState::namespace`). `tenant` is
    /// the request's tenant identifier when one is carried (cloud); it is ignored
    /// under a single-namespace profile. On a per-tenant profile a present tenant
    /// yields its dedicated namespace; an absent tenant falls back to `configured`.
    ///
    /// # Errors
    /// [`ProfileError::TenantRequired`] when this is a multi-tenant profile but the
    /// tenant identifier is empty — cloud must not silently collapse a missing
    /// tenant onto a shared namespace.
    pub fn resolve_namespace(
        &self,
        configured: &str,
        tenant: Option<&str>,
    ) -> Result<String, ProfileError> {
        match self.namespace_strategy {
            NamespaceStrategy::Single => Ok(configured.to_owned()),
            NamespaceStrategy::PerTenant => match tenant.map(str::trim) {
                Some(t) if !t.is_empty() => Ok(tenant_namespace(t)),
                _ => Err(ProfileError::TenantRequired),
            },
        }
    }
}

/// Derive a per-tenant namespace name from a tenant identifier.
///
/// A stable, collision-free mapping is all the gate needs: the tenant id is the
/// namespace suffix, so tenant `acme` signs into `tenant_acme`. Kept in one place
/// so the boot resolution and any future admin tooling agree on the name.
fn tenant_namespace(tenant: &str) -> String {
    format!("tenant_{tenant}")
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cloud")]
    use super::super::cloud;
    #[cfg(feature = "edge")]
    use super::super::edge;

    #[cfg(feature = "edge")]
    #[test]
    fn edge_resolves_to_the_configured_namespace_ignoring_any_tenant() {
        let p = edge::profile();
        assert_eq!(p.resolve_namespace("rubix", None).unwrap(), "rubix");
        assert_eq!(p.resolve_namespace("rubix", Some("acme")).unwrap(), "rubix");
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_resolves_a_per_tenant_namespace() {
        let p = cloud::profile();
        assert_eq!(
            p.resolve_namespace("rubix", Some("acme")).unwrap(),
            "tenant_acme"
        );
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_rejects_a_request_with_no_tenant() {
        use super::ProfileError;
        let p = cloud::profile();
        assert!(matches!(
            p.resolve_namespace("rubix", None).unwrap_err(),
            ProfileError::TenantRequired
        ));
    }
}
