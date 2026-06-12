//! Dev-gated portfolio seed: populate a fresh store with the demo building
//! portfolio (4 sites × 9 equips × the point blueprint, with populated priority
//! arrays, 7-day history backfill, and per-site sparks) as **real rows** through
//! the store layer, so the UI renders the operator-approved populated look with
//! zero fake data.
//!
//! Activated by `--seed-dev` on `rubix-server` (never the default). Idempotent:
//! re-running upserts by slug. See `docs/sessions/ui/UI-02.md`.

mod curves;
mod portfolio;
mod tags;
mod write;

pub use write::{seed_portfolio, SeedReport};

use crate::store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("blueprint references unknown equip `{0}`")]
    MissingEquip(&'static str),
    #[error("blueprint references unknown point `{0}`")]
    MissingPoint(&'static str),
}
