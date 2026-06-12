//! Running SQL against the query engine and shaping results.

mod run;

use serde_json::Value;

/// The rows produced by a query, as a JSON array of objects (column -> value).
///
/// This is the wire shape returned to dashboards and handed to reflow actors
/// and awaken tools. Arrow types are mapped by `arrow-json`'s writer.
pub type QueryRows = Vec<Value>;
