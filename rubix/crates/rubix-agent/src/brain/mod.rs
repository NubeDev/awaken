//! The agent's brain — the Rig LLM seam (AGENT.md primary design).
//!
//! AGENT.md's thesis is "import the brain (Rig), wire its seams to the substrate":
//! the agent loop, the `Tool` trait, and the `VectorStoreIndex` trait are Rig's,
//! and rubix owns only the two seams that must cross the gate (memory + tools).
//! This module is the thin `Provider` seam — it constructs Rig's OpenAI client and
//! exposes the two operations the rest of the runtime needs:
//!
//! - [`Brain::embed`] turns text into an embedding, the **input to the existing
//!   memory seam**: its `Vec<f64>` flows straight into
//!   [`MemoryRecord::new`](crate::MemoryRecord) (normalized on write) and into
//!   [`recall_memory`](crate::recall_memory) as a probe. Rig's
//!   [`Embedding::vec`](rig::embeddings::embedding::Embedding) is already
//!   `Vec<f64>`, so there is no glue and no second embedding path.
//! - [`Brain::answer`] runs a Rig [`Agent`](rig::agent::Agent) — preamble in,
//!   prompt in, answer out — over the chosen completion model.
//!
//! The brain is **provider wiring only**; it never touches SurrealDB, the gate, or
//! a scoped session. That keeps the safety thesis intact: the brain proposes text
//! and embeddings, but every *read* still runs on the scoped session and every
//! *write* still crosses the gate (the memory seam, unchanged). The brain sits
//! behind the `cloud` feature so the edge build carries no cloud provider and
//! fails closed when one is requested but absent (AGENT.md, open question 1).

use rig::client::{CompletionClient, EmbeddingsClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel;
use rig::providers::openai;

use crate::error::{AgentError, Result};

/// The default OpenAI completion model the agent reasons with.
///
/// A current, capable default ([model constants](rig::providers::openai)); a
/// caller can override per [`BrainConfig`]. The brain does not hard-code a tier —
/// the model is config, not policy.
pub const DEFAULT_COMPLETION_MODEL: &str = openai::GPT_5_2;

/// The default OpenAI embedding model used for memory.
///
/// `text-embedding-3-small` (1536 dims) — the embeddings are L2-normalized by the
/// memory seam on write, so euclidean recall ranks them the way cosine would
/// (AGENT.md, "Memory schema"; open question 3c). Swapping the model only changes
/// the dimensionality stored, not the normalize-then-euclidean contract.
pub const DEFAULT_EMBEDDING_MODEL: &str = openai::TEXT_EMBEDDING_3_SMALL;

/// How to construct the brain: the API key and the models it should use.
///
/// The key is supplied explicitly rather than read from the ambient environment
/// so the server can source it from its own config/secret store and so tests can
/// inject one — the brain never silently reaches for `OPENAI_API_KEY`. An optional
/// `base_url` points the client at a compatible gateway/proxy (the same override
/// Rig's `from_env` honors); leave it `None` for OpenAI itself.
#[derive(Debug, Clone)]
pub struct BrainConfig {
    /// The OpenAI (or compatible) API key.
    pub api_key: String,
    /// Override base URL for an OpenAI-compatible endpoint; `None` uses OpenAI.
    pub base_url: Option<String>,
    /// The completion model the agent reasons with.
    pub completion_model: String,
    /// The embedding model memory is embedded with.
    pub embedding_model: String,
}

impl BrainConfig {
    /// A config from an API key alone, taking the default models.
    ///
    /// The common case: the deployment has a key and wants the recommended
    /// completion/embedding pair. Override the model fields for a different tier.
    #[must_use]
    pub fn from_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            completion_model: DEFAULT_COMPLETION_MODEL.to_owned(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_owned(),
        }
    }
}

/// The agent's LLM brain over Rig's OpenAI provider.
///
/// Holds the constructed Rig client and the model names; cloneable so a server
/// handler can hold one in shared state and use it per request. Construction does
/// no network I/O — the key is only exercised when [`embed`](Brain::embed) or
/// [`answer`](Brain::answer) is called, so a missing/invalid key surfaces as an
/// [`AgentError::Brain`] at use, not at boot.
#[derive(Clone)]
pub struct Brain {
    client: openai::Client,
    completion_model: String,
    embedding_model: String,
}

impl Brain {
    /// Construct the brain from `config`.
    ///
    /// Builds Rig's OpenAI client with the explicit key (and optional base URL),
    /// mirroring Rig's own `from_env` construction but sourcing the key from
    /// rubix config rather than the process environment.
    ///
    /// # Errors
    /// Returns [`AgentError::Brain`] if the client cannot be built (an invalid
    /// base URL or header).
    pub fn new(config: &BrainConfig) -> Result<Self> {
        let mut builder = openai::Client::builder().api_key(&config.api_key);
        if let Some(base) = &config.base_url {
            builder = builder.base_url(base);
        }
        let client = builder
            .build()
            .map_err(|e| AgentError::Brain(format!("could not build OpenAI client: {e}")))?;
        Ok(Self {
            client,
            completion_model: config.completion_model.clone(),
            embedding_model: config.embedding_model.clone(),
        })
    }

    /// Embed `text` into a vector suitable for the memory seam.
    ///
    /// The returned `Vec<f64>` is fed directly to
    /// [`MemoryRecord::new`](crate::MemoryRecord) (persist) or
    /// [`recall_memory`](crate::recall_memory) (probe), which L2-normalize it — so
    /// the brain returns the raw provider embedding and the memory seam owns the
    /// geometry. No normalization happens here, on purpose: one normalize step, in
    /// the seam that stores the vector.
    ///
    /// # Errors
    /// Returns [`AgentError::Brain`] if the provider rejects the request (missing
    /// or invalid key, transport failure) or returns no embedding.
    pub async fn embed(&self, text: &str) -> Result<Vec<f64>> {
        let model = self.client.embedding_model(&self.embedding_model);
        let embedding = model
            .embed_text(text)
            .await
            .map_err(|e| AgentError::Brain(format!("embedding failed: {e}")))?;
        Ok(embedding.vec)
    }

    /// Answer `prompt` as an agent with `preamble` as its system instruction.
    ///
    /// Builds a Rig [`Agent`](rig::agent::Agent) on the completion model and runs
    /// one prompt turn. The `preamble` is where the caller injects grounding —
    /// recalled memory, the site's records, the agent's tier — so the answer is
    /// conditioned on what the principal may see, assembled by the caller on the
    /// scoped session before the brain is ever called.
    ///
    /// This is the no-tools path (a plain Q&A turn). Tool-calling — each tool
    /// fronted by the capability bridge so the LLM cannot reach an ungranted plane
    /// — is the `Tool`-seam follow-on; it attaches `.tool(..)` to this same
    /// builder (AGENT.md, "Seam mapping").
    ///
    /// # Errors
    /// Returns [`AgentError::Brain`] if the provider rejects the request or the
    /// completion fails.
    pub async fn answer(&self, preamble: &str, prompt: &str) -> Result<String> {
        let agent = self
            .client
            .agent(&self.completion_model)
            .preamble(preamble)
            .build();
        agent
            .prompt(prompt)
            .await
            .map_err(|e| AgentError::Brain(format!("completion failed: {e}")))
    }
}
