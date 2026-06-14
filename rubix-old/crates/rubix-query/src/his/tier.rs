//! The Parquet cold-tier handle: a local-filesystem object store rooted at a
//! configured directory.
//!
//! Edge nodes tier `his` to a local Parquet directory; the same handle backs
//! both the union read path ([`HisTable`](super::HisTable)) and the flush write
//! path ([`write_partitions`](super::write_partitions)). A cloud/remote object
//! store is a future variant — the read/write code already takes a generic
//! `ObjectStore`, so only construction changes.

use std::path::Path;
use std::sync::Arc;

use datafusion::object_store::local::LocalFileSystem;
use datafusion::object_store::ObjectStore;

use crate::error::QueryError;

/// A handle to the Parquet cold tier. Cheap to clone (the store is `Arc`-shared).
#[derive(Clone)]
pub struct HisTier {
    store: Arc<dyn ObjectStore>,
}

impl HisTier {
    /// Open the cold tier rooted at the local directory `root`, creating it if
    /// absent. Partition paths are relative to this root.
    pub fn open_local(root: &Path) -> Result<Self, QueryError> {
        std::fs::create_dir_all(root)
            .map_err(|e| QueryError::His(format!("create his tier dir {}: {e}", root.display())))?;
        let store = LocalFileSystem::new_with_prefix(root)
            .map_err(|e| QueryError::His(format!("open his tier store: {e}")))?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    /// The underlying object store, shared by the read and flush paths.
    pub fn store(&self) -> Arc<dyn ObjectStore> {
        self.store.clone()
    }
}
