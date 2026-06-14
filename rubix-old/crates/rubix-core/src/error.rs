use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("priority {0} out of range 1..=16")]
    PriorityOutOfRange(u8),
    #[error("invalid slug `{0}`: must be non-empty lowercase [a-z0-9-]")]
    InvalidSlug(String),
    #[error("invalid tag `{0}`: must be non-empty lowercase [a-zA-Z0-9_]")]
    InvalidTag(String),
    #[error("invalid nav node: {0}")]
    InvalidNavNode(String),
}
