//! Open the zenoh session backing the data plane.

use std::sync::Arc;

use zenoh::Session;

use super::ZenohBus;
use crate::store::Store;

impl ZenohBus {
    /// Open a zenoh session with the default (peer) config and bind it to the
    /// store that answers `write`/`his` queries.
    pub async fn open(store: Store) -> anyhow::Result<Self> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(|e| anyhow::anyhow!("zenoh open: {e}"))?;
        Ok(Self {
            session: Arc::new(session),
            store,
        })
    }

    pub(super) fn session(&self) -> &Session {
        &self.session
    }
}
