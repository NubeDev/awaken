//! In-flight pre-processing nodes: decimate, filter, enrich.
//!
//! Raw high-rate streams are processed *before* persistence, not written first
//! and queried back (`rubix/docs/SCOPE.md`, "Ingestion and pre-processing"):
//! [`decimate`] cuts the rate, [`filter`] drops by predicate, [`enrich`] attaches
//! derived fields, and [`pipeline`] composes the three in order.

mod decimate;
mod enrich;
mod filter;
mod pipeline;

pub use decimate::Decimator;
pub use enrich::Enricher;
pub use filter::Filter;
pub use pipeline::Pipeline;
