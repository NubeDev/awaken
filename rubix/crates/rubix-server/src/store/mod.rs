//! Relational persistence. One file per resource; shared row/JSON codecs in
//! [`codec`] and [`point_row`]. The [`Store`] wraps a [`backend::Backend`]:
//! SQLite (edge default) or Postgres (cloud feature). Each resource method
//! dispatches on the backend; the SQLite body stays here, the Postgres body
//! lives in the [`postgres`] submodule. STACK-DEISGN.md "Postgres (cloud),
//! SQLite (edge)".

mod backend;
mod boards;
mod changes;
mod codec;
mod command;
mod dashboards;
mod entity_tags;
mod equips;
mod error;
mod grants;
mod his;
mod nav_nodes;
mod his_flush;
mod keyexpr;
mod migrate;
mod open;
mod point_row;
mod points;
mod prefs;
#[cfg(feature = "cloud")]
mod postgres;
mod reversible;
mod rules;
mod runs;
mod schema;
mod sites;
mod sparks;
mod teams;
mod tokens;
mod users;
mod widgets;

pub use changes::{new_change_id, new_group_id, ChangeFilter, UndoCursor};
pub use error::StoreError;
pub use grants::{GrantRecord, Permission, SubjectKind};
pub use reversible::{
    apply_group_forward, apply_group_inverse, registered_kinds, Reversible, ReverserRegistry,
};
pub use keyexpr::PointKey;
pub use open::Store;
pub use rules::RuleRecord;
pub use teams::TeamRecord;
pub use users::UserRecord;

pub(crate) type Result<T> = std::result::Result<T, StoreError>;
