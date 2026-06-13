//! Query-layer errors.

use thiserror::Error;

/// Failure building the query context or running a statement.
#[derive(Debug, Error)]
pub enum QueryError {
    /// The SQLite connection pool could not be opened.
    #[error("open sqlite pool: {0}")]
    Pool(String),

    /// A SQLite read (schema or rows) failed.
    #[error("sqlite backend: {0}")]
    Backend(String),

    /// A tenant scope was malformed (empty or carrying an illegal character).
    #[error("tenant scope: {0}")]
    Scope(String),

    /// A `his` Parquet cold-tier operation failed (open, list, read, or write).
    #[error("his parquet tier: {0}")]
    His(String),

    /// A canonical table could not be built into a provider.
    #[error("build provider for `{table}`: {message}")]
    Provider {
        table: &'static str,
        message: String,
    },

    /// A canonical table provider could not be registered in the context.
    #[error("register table `{table}`: {source}")]
    Register {
        table: &'static str,
        source: datafusion::error::DataFusionError,
    },

    /// The statement is not a single read-only `SELECT`/`WITH`. Writes are
    /// refused up front rather than relying on the read-only DataFusion
    /// providers to no-op them — a clear error and defense against a future
    /// mutable provider.
    #[error("only a single read-only SELECT/WITH statement is allowed")]
    NotReadOnly,

    /// A variable token could not be lowered into bound parameters (an unknown
    /// or unbound reference, or a misused single/multi token). Surfaced as a
    /// caller-correctable error, never a silent passthrough.
    #[error("interpolate variables: {0}")]
    Interpolate(#[from] crate::interpolate::InterpolateError),

    /// SQL planning or execution failed.
    #[error("execute sql: {0}")]
    Execute(#[from] datafusion::error::DataFusionError),

    /// A result batch could not be serialized to JSON rows.
    #[error("encode rows: {0}")]
    Encode(#[from] datafusion::arrow::error::ArrowError),

    /// The encoded JSON rows could not be decoded back into result values.
    #[error("decode rows: {0}")]
    Decode(#[from] serde_json::Error),
}
