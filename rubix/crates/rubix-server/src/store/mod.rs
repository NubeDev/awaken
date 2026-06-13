//! Relational persistence. One file per resource; shared row/JSON codecs in
//! [`codec`] and [`point_row`]. The [`Store`] wraps a [`backend::Backend`]:
//! SQLite (edge default) or Postgres (cloud feature). Each resource method
//! dispatches on the backend; the SQLite body stays here, the Postgres body
//! lives in the [`postgres`] submodule. STACK-DEISGN.md "Postgres (cloud),
//! SQLite (edge)".

mod backend;
mod boards;
mod codec;
mod command;
mod dashboards;
mod equips;
mod error;
mod grants;
mod his;
mod his_flush;
mod keyexpr;
mod migrate;
mod open;
mod point_row;
mod points;
#[cfg(feature = "cloud")]
mod postgres;
mod rules;
mod runs;
mod schema;
mod sites;
mod sparks;
mod teams;
mod tokens;
mod users;
mod widgets;

pub use error::StoreError;
pub use grants::{GrantRecord, Permission, SubjectKind};
pub use keyexpr::PointKey;
pub use open::Store;
pub use rules::RuleRecord;
pub use teams::TeamRecord;
pub use users::UserRecord;

pub(crate) type Result<T> = std::result::Result<T, StoreError>;
