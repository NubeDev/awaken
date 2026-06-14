//! The server-side `his` tiering: flushing aged SQLite rows into the Parquet
//! cold tier. The query-side union and partition layout live in `rubix-query`;
//! this module owns the write/retention boundary.

mod flush;

pub use flush::{flush_aged, FlushReport};
