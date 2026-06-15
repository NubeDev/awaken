//! `POST /agent/ask` — answer a free-form question with the agent brain.
//!
//! This is the conversational surface the Copilot UI calls (AGENT.md, "The agent
//! is the front door"). It runs the Rig brain ([`Brain::answer`](rubix_agent::Brain))
//! over the chosen provider, with the caller-supplied grounding folded into the
//! agent's preamble so the answer is conditioned on what the principal may see —
//! grounding the caller assembled on its scoped session before asking.
//!
//! It fails closed honestly: the brain is behind the build's `cloud` feature and
//! needs an `OPENAI_API_KEY`. When either is absent the route does **not**
//! fabricate an answer — it returns `grounded: false` and a model-free message, so
//! the UI can label a degraded answer rather than present a guess as the agent's
//! (AGENT.md, open question 1; the Copilot page already shows this distinction).

use axum::Json;

use crate::auth::Authenticated;
use crate::dto::agent::{AskRequest, AskResponse};
use crate::error::ApiResult;

/// The environment variable the cloud brain's API key is sourced from.
///
/// Sourced from the server's environment (not the request) so a key is never
/// shipped per call; the same variable Rig's own `from_env` reads, kept here so
/// the brain is constructed from rubix config rather than reaching for the ambient
/// environment itself.
#[cfg(feature = "cloud")]
const API_KEY_ENV: &str = "OPENAI_API_KEY";

/// `POST /agent/ask` — answer `question`, grounded by optional `context`.
pub async fn ask_agent_route(
    auth: Authenticated,
    Json(body): Json<AskRequest>,
) -> ApiResult<Json<AskResponse>> {
    let _ = &auth; // the answer is the principal's; tool-calling will use it next.
    Ok(Json(answer(&body).await))
}

/// Produce the answer, using the cloud brain when available and falling back to a
/// grounded, model-free reply otherwise.
#[cfg(feature = "cloud")]
async fn answer(body: &AskRequest) -> AskResponse {
    use rubix_agent::{Brain, BrainConfig};

    let Ok(api_key) = std::env::var(API_KEY_ENV) else {
        return grounded_fallback(body, "no model is configured");
    };
    let brain = match Brain::new(&BrainConfig::from_api_key(api_key)) {
        Ok(brain) => brain,
        Err(e) => return grounded_fallback(body, &e.to_string()),
    };

    match brain.answer(&preamble(body), &body.question).await {
        Ok(answer) => AskResponse {
            answer,
            grounded: true,
        },
        Err(e) => grounded_fallback(body, &e.to_string()),
    }
}

/// The edge build carries no cloud provider, so the answer is always the grounded
/// fallback — fail closed, never a fabricated model answer.
#[cfg(not(feature = "cloud"))]
async fn answer(body: &AskRequest) -> AskResponse {
    grounded_fallback(body, "this build has no cloud brain")
}

/// Build the agent's system preamble from the request's grounding.
///
/// The grounding the caller assembled on its scoped session becomes the agent's
/// context, so the brain answers over what the principal may see rather than its
/// training prior alone. Kept small and explicit — the preamble is the only place
/// untrusted-to-the-model context enters this turn.
#[cfg(feature = "cloud")]
fn preamble(body: &AskRequest) -> String {
    let mut preamble = String::from(
        "You are Rubix, an assistant embedded in a building-data platform. \
         Answer concisely and only from the context provided; if the context does \
         not contain the answer, say so plainly rather than guessing.",
    );
    if let Some(context) = &body.context {
        if !context.trim().is_empty() {
            preamble.push_str("\n\nContext:\n");
            preamble.push_str(context);
        }
    }
    preamble
}

/// A model-free answer that names why the brain was unavailable.
///
/// Honest degradation: it does not pretend to be the model's answer (`grounded:
/// false`), so the UI can show it as a fallback. If the caller passed grounding we
/// echo a short acknowledgement of it; otherwise we point at the limitation.
fn grounded_fallback(body: &AskRequest, reason: &str) -> AskResponse {
    let question = body.question.trim();
    let answer = match &body.context {
        Some(context) if !context.trim().is_empty() => format!(
            "I can't reach the language model right now ({reason}), so I can't \
             reason over “{question}” — but here is the context I was given:\n\n{context}"
        ),
        _ => format!(
            "I can't reach the language model right now ({reason}), so I can't answer \
             “{question}”. Ask an admin to configure the agent brain, or query records \
             directly in the meantime."
        ),
    };
    AskResponse {
        answer,
        grounded: false,
    }
}
