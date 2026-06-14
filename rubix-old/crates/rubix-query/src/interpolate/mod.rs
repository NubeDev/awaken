//! Injection-safe SQL variable interpolation.
//!
//! Rubix has no SQL macro/variable binder; this module is it (see
//! docs/design/variables-and-templating.md §2). It lowers `$name` / `${name}` /
//! `${name:csv}` / `${name:singlequote}` / `$__sqlIn(name)` tokens in SQL text
//! into `$N` positional placeholders plus an ordered list of bound parameters,
//! shared by both query paths (the DataFusion `/query` route and the datasource
//! `/query` route). Every variable value leaves as a bound parameter, never
//! spliced into the SQL text — that is the security boundary this module owns.

mod bound;
mod error;
mod lower;
mod time;
mod time_macro;
mod var;

pub use bound::{BoundParam, Lowered};
pub use error::InterpolateError;
pub use lower::lower;
pub use time::{resolve as resolve_time_range, TimeContext, TimeRangeSpec};
pub use var::{QueryVariable, Scalar, VarValue};
