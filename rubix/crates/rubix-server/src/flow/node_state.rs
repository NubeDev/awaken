//! Host-side [`NodeState`] for a board run. Routes a node's state by its
//! [`StatePolicy`]:
//!
//! - **Ephemeral** → a per-engine in-memory map, created with the access, so it
//!   resets when the engine is rebuilt (a republish/save).
//! - **Session** → a process-wide, board-scoped map the scheduler owns, so it
//!   survives a republish but clears on a server restart.
//! - **Durable** → the store (`node_state` table), so it survives a restart.
//!
//! When a backing is absent (a one-shot Test Run wires no session map or board
//! id) the higher policies degrade to Ephemeral, so a stateful node still works,
//! fresh per run — which is the right behaviour for a one-shot run.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rubix_flow::{FlowAccessError, NodeState, StatePolicy};
use serde_json::Value;

use crate::store::Store;

/// Process-wide, board-scoped Session state, keyed by `(board_id, node, key)`.
/// Owned by the scheduler so it outlives an engine rebuild.
pub type SessionStore = Arc<Mutex<HashMap<(String, String, String), Value>>>;

/// Per-engine Ephemeral state, keyed by `(node, key)`. Created with the access,
/// so a new engine starts empty.
pub type EphemeralStore = Arc<Mutex<HashMap<(String, String), Value>>>;

/// The node-state handle handed to a board's nodes for one run.
pub struct BoardNodeState {
    ephemeral: EphemeralStore,
    session: Option<SessionStore>,
    store: Store,
    board_id: Option<String>,
}

impl BoardNodeState {
    pub fn new(
        ephemeral: EphemeralStore,
        session: Option<SessionStore>,
        store: Store,
        board_id: Option<String>,
    ) -> Self {
        Self {
            ephemeral,
            session,
            store,
            board_id,
        }
    }

    /// Resolve the policy actually serviceable given which backings are wired,
    /// degrading toward Ephemeral when a higher policy's backing is absent.
    fn effective(&self, policy: StatePolicy) -> StatePolicy {
        match policy {
            StatePolicy::Durable if self.board_id.is_some() => StatePolicy::Durable,
            StatePolicy::Session | StatePolicy::Durable
                if self.session.is_some() && self.board_id.is_some() =>
            {
                StatePolicy::Session
            }
            _ => StatePolicy::Ephemeral,
        }
    }
}

#[async_trait]
impl NodeState for BoardNodeState {
    async fn load(
        &self,
        node: &str,
        key: &str,
        policy: StatePolicy,
    ) -> Result<Option<Value>, FlowAccessError> {
        match self.effective(policy) {
            StatePolicy::Durable => {
                let store = self.store.clone();
                let board = self.board_id.clone().unwrap_or_default();
                let (node, key) = (node.to_string(), key.to_string());
                tokio::task::spawn_blocking(move || store.get_node_state(&board, &node, &key))
                    .await
                    .map_err(|e| FlowAccessError::Store(format!("node_state task: {e}")))?
                    .map_err(FlowAccessError::store)
            }
            StatePolicy::Session => {
                let board = self.board_id.clone().unwrap_or_default();
                let map = self
                    .session
                    .as_ref()
                    .expect("session backing present")
                    .lock()
                    .expect("session state poisoned");
                Ok(map
                    .get(&(board, node.to_string(), key.to_string()))
                    .cloned())
            }
            StatePolicy::Ephemeral => {
                let map = self.ephemeral.lock().expect("ephemeral state poisoned");
                Ok(map.get(&(node.to_string(), key.to_string())).cloned())
            }
        }
    }

    async fn save(
        &self,
        node: &str,
        key: &str,
        policy: StatePolicy,
        value: Value,
    ) -> Result<(), FlowAccessError> {
        match self.effective(policy) {
            StatePolicy::Durable => {
                let store = self.store.clone();
                let board = self.board_id.clone().unwrap_or_default();
                let (node, key) = (node.to_string(), key.to_string());
                tokio::task::spawn_blocking(move || store.set_node_state(&board, &node, &key, &value))
                    .await
                    .map_err(|e| FlowAccessError::Store(format!("node_state task: {e}")))?
                    .map_err(FlowAccessError::store)
            }
            StatePolicy::Session => {
                let board = self.board_id.clone().unwrap_or_default();
                self.session
                    .as_ref()
                    .expect("session backing present")
                    .lock()
                    .expect("session state poisoned")
                    .insert((board, node.to_string(), key.to_string()), value);
                Ok(())
            }
            StatePolicy::Ephemeral => {
                self.ephemeral
                    .lock()
                    .expect("ephemeral state poisoned")
                    .insert((node.to_string(), key.to_string()), value);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> Store {
        let dir = Box::leak(Box::new(tempfile::tempdir().expect("tempdir")));
        Store::open(&dir.path().join("ns.db")).expect("store")
    }

    fn fresh_ephemeral() -> EphemeralStore {
        Arc::new(Mutex::new(HashMap::new()))
    }

    /// A rebuild (new ephemeral map, same session map + board id) keeps Session
    /// state but discards Ephemeral state — the trigger-survives-save fix.
    #[tokio::test]
    async fn session_survives_rebuild_ephemeral_resets() {
        let store = store();
        let session: SessionStore = Arc::new(Mutex::new(HashMap::new()));
        let board = Some("board-1".to_string());

        let before =
            BoardNodeState::new(fresh_ephemeral(), Some(session.clone()), store.clone(), board.clone());
        before
            .save("n", "k", StatePolicy::Session, json!(1))
            .await
            .unwrap();
        before
            .save("n", "e", StatePolicy::Ephemeral, json!(9))
            .await
            .unwrap();

        // Rebuild: a brand-new engine — fresh ephemeral, the same shared session.
        let after =
            BoardNodeState::new(fresh_ephemeral(), Some(session.clone()), store.clone(), board);
        assert_eq!(
            after.load("n", "k", StatePolicy::Session).await.unwrap(),
            Some(json!(1)),
            "Session state survives the rebuild"
        );
        assert_eq!(
            after.load("n", "e", StatePolicy::Ephemeral).await.unwrap(),
            None,
            "Ephemeral state is gone after the rebuild"
        );
    }

    /// Durable state round-trips through the store, surviving a rebuild with a
    /// fresh session map too (i.e. it would survive a restart).
    #[tokio::test]
    async fn durable_survives_via_the_store() {
        let store = store();
        let board = Some("board-1".to_string());

        let before = BoardNodeState::new(fresh_ephemeral(), None, store.clone(), board.clone());
        before
            .save("n", "k", StatePolicy::Durable, json!({"count": 7}))
            .await
            .unwrap();

        // Fresh everything except the store (as after a restart).
        let after = BoardNodeState::new(fresh_ephemeral(), None, store.clone(), board);
        assert_eq!(
            after.load("n", "k", StatePolicy::Durable).await.unwrap(),
            Some(json!({"count": 7})),
            "Durable state survives via the store"
        );
    }

    /// With no session map / board id wired, Session degrades to Ephemeral —
    /// fresh per run, so a one-shot Test Run does not leak state across runs.
    #[tokio::test]
    async fn session_degrades_to_ephemeral_without_backing() {
        let store = store();
        let ns = BoardNodeState::new(fresh_ephemeral(), None, store, None);
        ns.save("n", "k", StatePolicy::Session, json!(1))
            .await
            .unwrap();
        // A rebuild (fresh ephemeral) loses it, since there is no session backing.
        let rebuilt = BoardNodeState::new(fresh_ephemeral(), None, ns.store.clone(), None);
        assert_eq!(
            rebuilt.load("n", "k", StatePolicy::Session).await.unwrap(),
            None
        );
    }
}
