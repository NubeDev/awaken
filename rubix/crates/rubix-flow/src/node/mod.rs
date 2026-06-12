//! Custom rubix reflow nodes. Each node hand-implements [`reflow_actor::Actor`]
//! over [`actor_base::ActorBase`] so it can hold an injected
//! [`crate::port::PointAccess`].

pub mod actor_base;
mod agent_call;
mod emit_spark;
mod query_his;
mod read_point;
mod value_msg;
mod write_point;

pub use agent_call::AgentCallActor;
pub use emit_spark::EmitSparkActor;
pub use query_his::QueryHisActor;
pub use read_point::ReadPointActor;
pub use write_point::WritePointActor;
