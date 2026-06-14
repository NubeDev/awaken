//! Write-source gating: agents are confined to low-priority slots.

use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteSource {
    #[default]
    Operator,
    /// AI/agent writes are restricted to low-priority slots; an operator
    /// command always wins.
    Agent,
}

pub(crate) fn check_agent_priority(
    state: &AppState,
    source: WriteSource,
    priority: u8,
) -> Result<(), ApiError> {
    if source == WriteSource::Agent && priority < state.ai_min_priority {
        return Err(ApiError::Forbidden(format!(
            "agent writes are limited to priority {} or lower (requested {priority})",
            state.ai_min_priority
        )));
    }
    Ok(())
}
