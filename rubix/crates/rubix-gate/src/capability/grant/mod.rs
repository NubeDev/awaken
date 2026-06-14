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
mod model;
mod revoke;
pub(crate) mod row;

pub use create::create_grant;
pub use list::list_grants;
pub use model::Grant;
pub use revoke::revoke_grant;
