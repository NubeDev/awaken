//! Driver-contract errors: manifest validation and capability denials.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DriverError {
    #[error("invalid driver manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid keyexpr prefix `{0}`: {1}")]
    InvalidPrefix(String, &'static str),
    #[error("driver `{driver}` is not granted `{action}` on `{key}`")]
    Denied {
        driver: String,
        action: &'static str,
        key: String,
    },
}
