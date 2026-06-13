//! Row/byte/wall-clock bounds on a single datasource read.

mod admit;
mod limit;

pub use admit::CapState;
pub use limit::Caps;
