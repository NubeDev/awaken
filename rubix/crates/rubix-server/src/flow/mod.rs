//! reflow engine integration: implements [`rubix_flow::PointAccess`] over the
//! store so control/rule boards read and command real points.

mod access;

pub use access::StorePointAccess;
