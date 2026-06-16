//! `POST /agent` — provision an AI agent as a scoped service-account principal.
//!
//! Provisioning an agent is an admin action (AGENT.md, "Grant administration stays
//! a human-admin action"): the caller must be an `Admin` in the namespace, and the
//! agent is registered as an `Extension`-kind principal granted its
//! [`AgentTier`](rubix_agent::AgentTier) through the WS-04 grant path. The handler
//! mirrors the principal surface — subject is namespace-prefixed, a secret is
//! minted when omitted and returned exactly once. The agent can never escalate its
//! own tier: the grantor is the human admin, not the agent.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use rubix_agent::provision_agent;
use rubix_core::Id;

use crate::auth::Authenticated;
use crate::dto::admin::prefix_subject;
use crate::dto::agent::{ProvisionAgentRequest, ProvisionedAgentDto, parse_tier, tier_str};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::super::admin::guard::require_admin;

/// `POST /agent` — provision a new agent in the caller's namespace at a tier.
///
/// The secret may be caller-supplied or omitted; when omitted the server mints one
/// and returns it once. A supplied secret is never echoed back.
pub async fn provision_agent_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<ProvisionAgentRequest>,
) -> ApiResult<(StatusCode, Json<ProvisionedAgentDto>)> {
    let namespace = require_admin(&auth.principal)?;
    let tier = parse_tier(&body.tier)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown agent tier `{}`", body.tier)))?;

    let full_subject = prefix_subject(&namespace, &body.subject);

    // Mint a secret when the caller did not supply one; the minted secret is the
    // only one ever returned on the wire (the same rule the principal surface uses).
    let (secret, generated) = match body.secret {
        Some(s) => (s, None),
        None => {
            let minted = Id::new().to_string();
            (minted.clone(), Some(minted))
        }
    };

    let agent = provision_agent(
        state.store.raw(),
        &auth.principal,
        full_subject,
        namespace.clone(),
        secret,
        tier,
    )
    .await
    .map_err(map_provision_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ProvisionedAgentDto {
            subject: body.subject,
            namespace,
            tier: tier_str(agent.tier()).to_owned(),
            secret: generated,
        }),
    ))
}

/// Map an agent provisioning failure to its transport status.
///
/// A provisioning failure is either a grant denial (the caller lacks the authority
/// to confer the tier — `403`) or a write/identity failure (`500`). The agent
/// crate flattens both into [`AgentError::Provision`](rubix_agent::AgentError), so
/// we surface the message and treat it as a server-side failure unless it reads as
/// a denial.
pub(crate) fn map_provision_error(error: rubix_agent::AgentError) -> ApiError {
    let message = error.to_string();
    if message.contains("denied") || message.contains("not authorized") || message.contains("admin")
    {
        ApiError::Forbidden(message)
    } else {
        ApiError::Internal(message)
    }
}
