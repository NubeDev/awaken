//! In-process control plane: tokio broadcast fan-out, one channel per event
//! type.
//!
//! Component-to-component control events inside the binary, with no
//! serialization and no network (`rubix/docs/SCOPE.md`, "Event bus"). A
//! [`ControlBus`] is cloned and shared across components; [`publish`] fans an
//! event out to every [`subscribe`]r of its type and to no other type.

mod publish;
mod registry;
mod subscribe;

pub use publish::publish;
pub use registry::ControlBus;
pub use subscribe::{ControlSubscription, subscribe};
