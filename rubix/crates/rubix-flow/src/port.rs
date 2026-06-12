//! The engine's view of the BMS. reflow actors depend on this trait, not on
//! the server's store/bus directly — keeping `rubix-flow` free of axum, sqlite,
//! and zenoh. `rubix-server` implements it; tests provide a fake.
//!
//! Point writes always go through the priority array (never a raw set), per
//! STACK-DEISGN.md: "point write (always through the priority array, never
//! raw)".

use rubix_core::{HisSample, PointValue};

/// Read/command/query access to points, addressed by zenoh keyexpr prefix
/// (`{org}/{site}/{equip-path}/{point}`) so graphs reference points the same
/// way the bus and tags do.
pub trait PointAccess: Send + Sync + 'static {
    /// Current effective value of a point, or `None` if unset/unknown.
    fn read_point(&self, keyexpr: &str) -> anyhow::Result<Option<PointValue>>;

    /// Command a priority slot (1..=16). Returns the new effective value.
    fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>>;

    /// History samples for a point, most recent first, capped at `limit`.
    fn query_his(&self, keyexpr: &str, limit: usize) -> anyhow::Result<Vec<HisSample>>;
}
