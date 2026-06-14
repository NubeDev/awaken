//! Edge/cloud deployment profiles. STACK-DEISGN.md "Single binary, two
//! profiles": one executable, cargo features (`edge`/`cloud`) decide what is
//! compilable, `RUBIX_PROFILE` selects among the compiled options at boot, and
//! [`Profile`] carries the per-profile defaults threaded into `AppState`.

mod config;
mod kind;
mod select;

pub use config::{HistoryTier, Profile, StoreKind};
pub use kind::{ProfileKind, UnknownProfile};
pub use select::{resolve, select, ProfileError};
