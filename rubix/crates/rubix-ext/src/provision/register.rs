//! Create the extension's service-account principal (WS-03).
//!
//! Registering an extension is provisioning a [`Principal`](rubix_core::Principal)
//! whose [`kind`](rubix_core::PrincipalKind) is `Extension` — the *same*
//! identity write a user goes through (`rubix/docs/sessions/WS-13.md`, SCOPE
//! "Extensions as principals"). There is no separate plugin-trust path: the
//! extension authenticates with a subject/secret token and signs in to a scoped
//! SurrealDB session exactly as a user, so its data reads are confined to its
//! namespace by the engine's row-level permissions (contract #1). The role is
//! capped at [`Operator`](rubix_core::Role): an extension is a service account
//! that performs granted actions, never an administrator of grants — that keeps
//! grant administration a human-admin action and blocks an extension from
//! escalating its own authority.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::provision_principal;

use crate::error::{ExtError, Result};

/// An extension provisioned as a scoped service-account principal.
///
/// Carries the [`Principal`] identity (kind `Extension`) the rest of the crate
/// authorizes and audits against. The secret it was provisioned with is what the
/// extension later presents in its
/// [`PrincipalToken`](rubix_gate::PrincipalToken) to authenticate — the
/// registration does not retain it.
#[derive(Debug, Clone)]
pub struct ExtensionRegistration {
    /// The service-account principal the extension authenticates as.
    pub principal: Principal,
}

impl ExtensionRegistration {
    /// The extension's principal identity.
    #[must_use]
    pub fn principal(&self) -> &Principal {
        &self.principal
    }
}

/// Provision an extension `subject` in `namespace` with `secret`.
///
/// Builds an `Extension`-kind [`Principal`] at the [`Operator`](Role::Operator)
/// band and writes it through the WS-03 [`provision_principal`] path on the root
/// handle (provisioning is an owner action). The extension can then authenticate
/// with `subject`/`secret` and sign in to a namespace-scoped session like any
/// user. Granting it capabilities is a separate step
/// ([`grant_extension`](super::grant_extension)) — registration alone confers no
/// cross-plane authority (fail closed by default).
///
/// # Errors
/// Returns [`ExtError::Provision`] if the identity write fails.
pub async fn register_extension(
    db: &Surreal<Db>,
    subject: impl Into<String>,
    namespace: impl Into<String>,
    secret: impl Into<String>,
) -> Result<ExtensionRegistration> {
    let principal = Principal::new(
        Id::from_raw(subject),
        namespace,
        PrincipalKind::Extension,
        Role::Operator,
    );
    provision_principal(db, &principal, secret)
        .await
        .map_err(|e| ExtError::Provision(e.to_string()))?;
    Ok(ExtensionRegistration { principal })
}
