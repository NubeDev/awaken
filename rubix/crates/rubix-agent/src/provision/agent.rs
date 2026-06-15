//! Provision an AI agent as a scoped service-account principal at a tier.
//!
//! The agent is **not** a new identity model: it is provisioned exactly as any
//! extension is (`rubix/docs/design/AGENT.md`, "provisioned as a SCOPED
//! PRINCIPAL"; SCOPE principle 5) — a [`Principal`](rubix_core::Principal) of
//! kind `Extension`, bound to one namespace, authenticating with a subject/secret
//! and signing in to a scoped session like any user. This reuses the WS-03/WS-04
//! [`register_extension`]/[`grant_extension`] paths rather than defining a
//! parallel agent-trust path: the only agent-specific decision is the
//! [`AgentTier`] it is granted, which resolves to a WS-04 [`GrantProfile`]. Grant
//! administration stays a human-admin action — the `grantor` must already be a
//! namespace admin, so an agent can never escalate its own authority.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_ext::{grant_extension, register_extension};

use crate::error::{AgentError, Result};

use super::tier::AgentTier;

/// An agent provisioned as a scoped service-account principal at a tier.
///
/// Carries the [`Principal`] identity (kind `Extension`) the rest of the runtime
/// authorizes, audits, and scopes reads against. The secret it was provisioned
/// with is what the agent later presents to authenticate — the provisioning does
/// not retain it.
#[derive(Debug, Clone)]
pub struct AgentPrincipal {
    /// The service-account principal the agent authenticates as.
    principal: Principal,
    /// The tier the agent was granted.
    tier: AgentTier,
}

impl AgentPrincipal {
    /// The agent's principal identity.
    #[must_use]
    pub fn principal(&self) -> &Principal {
        &self.principal
    }

    /// The tier the agent was provisioned at.
    #[must_use]
    pub fn tier(&self) -> AgentTier {
        self.tier
    }
}

/// Provision an agent `subject` in `namespace` with `secret`, granted `tier`.
///
/// Registers the agent as an `Extension`-kind principal through the WS-03
/// [`register_extension`] path, then confers its [`AgentTier`]'s capabilities
/// through the WS-04 [`grant_extension`] path, authorized by `grantor` (which must
/// be a namespace admin — only a human admin may confer a grant, fail closed). On
/// success the agent can authenticate with `subject`/`secret` and act within its
/// tier; its data reads remain scoped by SurrealDB row-perms regardless of tier.
///
/// `db` is the store's owner handle (provisioning and granting are owner actions).
///
/// # Errors
/// Returns [`AgentError::Provision`] if the identity write fails or `grantor`
/// lacks the authority to confer the tier's grants. On a grant failure no further
/// grants are attempted (the underlying path stops at the first failure).
pub async fn provision_agent(
    db: &Surreal<Db>,
    grantor: &Principal,
    subject: impl Into<String>,
    namespace: impl Into<String>,
    secret: impl Into<String>,
    tier: AgentTier,
) -> Result<AgentPrincipal> {
    let registration = register_extension(db, subject, namespace, secret)
        .await
        .map_err(|e| AgentError::Provision(e.to_string()))?;
    let principal = registration.principal().clone();
    grant_extension(db, grantor, &principal, tier.profile())
        .await
        .map_err(|e| AgentError::Provision(e.to_string()))?;
    Ok(AgentPrincipal { principal, tier })
}
