//! Capability grants: the persisted unit of the app-enforced authz layer.
//!
//! A [`Grant`] attaches a [`Capability`](crate::capability::Capability) to a
//! principal within its namespace. The verbs administer grants through the gate
//! (`rubix/docs/SCOPE.md`, "applied through the gate"): `create` and `revoke`
//! are authority-checked and fail closed; `list` reads a principal's grants in
//! its own scope.

mod authority;
mod create;
mod list;
mod manage;
mod model;
mod revoke;
pub(crate) mod row;

pub use create::{create_grant, create_team_grant};
pub use list::{effective_grants, list_grants, list_team_grants};
pub use manage::{
    create_grant_audited, create_team_grant_audited, revoke_grant_audited,
    revoke_team_grant_audited,
};
pub use model::{Grant, TEAM_SUBJECT_PREFIX, team_subject};
pub use revoke::{revoke_grant, revoke_team_grant};
