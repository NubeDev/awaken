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
    /// A reliable (`write`/`his`) buffer is at capacity. Reliable channels never
    /// drop silently; backpressure surfaces here so the caller slows or fails
    /// rather than losing a command. See `docs/sessions/WS-10.md`.
    #[error("reliable buffer for `{key}` is full ({capacity} in flight); cannot enqueue")]
    BufferFull { key: String, capacity: usize },
    /// A reliable write was retried up to its bounded limit without an ack. The
    /// command is given up on (not retried forever, not silently lost) and the
    /// failure surfaces to the caller as a spark/error.
    #[error("write to `{key}` gave up after {attempts} attempts without ack")]
    AckTimeout { key: String, attempts: u32 },
}
