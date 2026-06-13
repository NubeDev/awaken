//! reflow engine integration: implements [`rubix_flow::PointAccess`] over the
//! store so control/rule boards read and command real points.

mod access;
mod rule_store;

pub use access::StorePointAccess;
pub use rule_store::TableRuleStore;
