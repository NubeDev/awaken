//! `GET /agent` — list the agents provisioned in the caller's namespace.
//!
//! Agents are not a separate identity model — they are `Extension`-kind principals
//! granted an agent tier (AGENT.md, "provisioned as a SCOPED PRINCIPAL"). So
//! listing agents is listing the namespace's extension principals and keeping the
//! ones whose grant set names an agent tier, inferring the tier from the
//! capabilities present. An extension with none of the agent capabilities is a
//! non-agent extension and is skipped — the list shows agents, fail closed on
//! ambiguity. Admin-guarded, like the principal surface it reads from.

use axum::Json;
use axum::extract::State;
use rubix_core::PrincipalKind;
use rubix_gate::{Capability, list_grants, list_principals};

use crate::auth::Authenticated;
use crate::dto::admin::strip_subject_prefix;
use crate::dto::agent::AgentDto;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::super::admin::guard::require_admin;

/// `GET /agent` — every provisioned agent in the caller's namespace.
pub async fn list_agents_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<AgentDto>>> {
    let namespace = require_admin(&auth.principal)?;

    let principals = list_principals(state.store.raw(), &namespace)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut agents = Vec::new();
    for principal in principals {
        if principal.kind != PrincipalKind::Extension {
            continue;
        }
        let grants = list_grants(state.store.raw(), &principal)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let capabilities: Vec<Capability> = grants.iter().map(|g| g.capability).collect();
        let Some(tier) = tier_from_capabilities(&capabilities) else {
            continue;
        };
        agents.push(AgentDto {
            subject: strip_subject_prefix(&principal.namespace, &principal.subject.to_string()),
            namespace: principal.namespace.clone(),
            tier: tier.to_owned(),
        });
    }

    Ok(Json(agents))
}

/// Infer an agent's tier from the capabilities it holds.
///
/// The tiers are layered (analyst ⊂ operator ⊂ actuator), so the distinguishing
/// capability of the highest tier wins: `device-actuate` ⇒ actuator, else
/// `rule-define` ⇒ operator, else `agent-memory-write` ⇒ analyst. An extension
/// holding none of these is not an agent (returns `None`). This mirrors the tier
/// definitions in `rubix-agent`'s `AgentTier` without depending on its private
/// grant-profile mapping.
fn tier_from_capabilities(capabilities: &[Capability]) -> Option<&'static str> {
    if capabilities.contains(&Capability::DeviceActuate) {
        Some("actuator")
    } else if capabilities.contains(&Capability::RuleDefine) {
        Some("operator")
    } else if capabilities.contains(&Capability::AgentMemoryWrite) {
        Some("analyst")
    } else {
        None
    }
}
