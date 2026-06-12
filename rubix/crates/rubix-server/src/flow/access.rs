//! [`PointAccess`] over the SQLite store: the bridge that lets reflow boards
//! read and command points by keyexpr. Writes go through the priority array
//! via [`Store::command_point`]; the agent-priority gate is enforced at the
//! HTTP/tool layer, not here (boards are operator-authored control logic).

use chrono::Utc;
use rubix_core::{HisSample, PointValue};
use rubix_flow::PointAccess;

use crate::store::Store;

/// Store-backed point access handed to [`rubix_flow::BoardGraph::load`].
#[derive(Clone)]
pub struct StorePointAccess {
    store: Store,
}

impl StorePointAccess {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl PointAccess for StorePointAccess {
    fn read_point(&self, keyexpr: &str) -> anyhow::Result<Option<PointValue>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        Ok(self.store.get_point(id)?.cur_value)
    }

    fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        let point = self
            .store
            .command_point(id, priority, Some(value), Utc::now())?;
        Ok(point.cur_value)
    }

    fn query_his(&self, keyexpr: &str, limit: usize) -> anyhow::Result<Vec<HisSample>> {
        let id = self.store.point_by_keyexpr(keyexpr)?;
        Ok(self.store.his_query(id, None, None, limit)?)
    }
}
