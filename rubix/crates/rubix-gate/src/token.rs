//! The bearer token a principal authenticates with.
//!
//! A token is the principal's subject plus a shared secret. `authenticate`
//! resolves it to a [`Principal`](rubix_core::Principal); `issue_session` signs
//! the scoped SurrealDB session in with the same pair. Keeping the token a plain
//! pair (not a JWT here) lets the SurrealDB record access method own credential
//! verification natively (`rubix/STACK-DEISGN.md`, contract #1/#2).

/// A principal's authentication token: subject identifier plus secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalToken {
    /// The principal's subject (the `principal` record key).
    pub subject: String,
    /// The shared secret proving the bearer is the principal.
    pub secret: String,
}

impl PrincipalToken {
    /// Build a token from a subject and secret.
    #[must_use]
    pub fn new(subject: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            secret: secret.into(),
        }
    }
}
