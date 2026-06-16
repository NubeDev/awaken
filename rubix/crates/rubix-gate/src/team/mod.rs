//! Teams and memberships: the grouping the authz layers grant access through.
//!
//! A [`Team`] is a named group of principals in a namespace; a [`Membership`]
//! links a principal to a team. They are gate-owned identity primitives
//! (`rubix/docs/SCOPE.md`, principle 5) administered through the gate: every
//! mutation is admin-authority-checked, fails closed, and is audited. The verbs
//! mirror the grant layer's shape. [`teams_of`] is the resolution the authz
//! layers build on so a grant to a team flows to its members.

mod manage;
mod model;
pub(crate) mod row;

pub use manage::{
    add_member, create_team, delete_team, get_team, list_members, list_teams, remove_member,
    teams_of,
};
pub use model::{Membership, Team};
