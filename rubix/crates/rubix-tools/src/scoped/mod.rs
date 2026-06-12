//! Scope-enforcing decorators over the tool access ports. A tenant-scoped run
//! wraps its access in these so every keyexpr-addressed call is confined to the
//! run's `{org}/{site}` at the tool boundary, independent of what the model asks
//! for.

mod point;

pub use point::ScopedPointAccess;
