//! Node state: a board-scoped, policy-driven store so a stateful node (a
//! trigger's clock, a counter, an integrator, an edge-detector, a debounce…)
//! does not have to invent its own persistence. A node author declares, at each
//! call, **how** the state should survive — there is no implicit default, so the
//! behaviour on a board republish/save is always an explicit decision.
//!
//! The contract is transport-free (like [`crate::PointAccess`]); the host backs
//! it. A state-less access (a test fake, the agent's own board access) returns no
//! store, and a node that needs one falls back to ephemeral actor memory.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::FlowAccessError;

/// How a node's retained state is maintained across a board **republish** (save,
/// which rebuilds the engine) and a **server restart**. There is no default — a
/// stateful node names its policy at each call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatePolicy {
    /// Reset on every engine rebuild (republish/save) and on restart. Scoped to a
    /// single started network. For state that must not outlive the exact graph it
    /// was computed under.
    Ephemeral,
    /// Survives a republish (in-memory, board+node scoped); cleared on restart.
    /// For runtime timers/counters that should not restart when the board is
    /// edited, but need not survive a deploy. (A `trigger`'s clock is `Session`.)
    Session,
    /// Survives a republish **and** a server restart (persisted to the store).
    /// For state that must be durable across deploys.
    Durable,
}

/// A node's handle to its own retained state, scoped to one board. The key is the
/// node id plus a node-chosen sub-key; the host routes storage by [`StatePolicy`].
///
/// `Value` is an opaque JSON blob the node serialises its own state into, so the
/// store stays node-agnostic. Reads return `None` when nothing was written under
/// that (node, key, policy).
#[async_trait]
pub trait NodeState: Send + Sync {
    /// Load the blob for `(node, key)` under `policy`, or `None` if unset.
    async fn load(
        &self,
        node: &str,
        key: &str,
        policy: StatePolicy,
    ) -> Result<Option<Value>, FlowAccessError>;

    /// Store the blob for `(node, key)` under `policy`.
    async fn save(
        &self,
        node: &str,
        key: &str,
        policy: StatePolicy,
        value: Value,
    ) -> Result<(), FlowAccessError>;
}
