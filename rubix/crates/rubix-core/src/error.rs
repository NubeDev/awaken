//! The project error model.
//!
//! A single `thiserror` enum shared across crates, with `.context()`-style
//! chaining so a low-level failure can be wrapped with the operation that was
//! being attempted (CLAUDE.md "Key Patterns").

use std::fmt;

/// Convenience alias for the project error.
pub type Result<T> = std::result::Result<T, Error>;

/// The project-wide error enum.
///
/// Domain crates surface their own typed failures (e.g. the store error) and
/// convert into this enum at crate boundaries. Variants stay coarse on purpose:
/// the human-facing detail lives in the message and the chained source.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A configuration value was missing or invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// A persistence/store operation failed.
    #[error("store error: {0}")]
    Store(String),

    /// An operation wrapped with the context in which it failed.
    #[error("{context}")]
    Context {
        /// What the caller was trying to do when the underlying error occurred.
        context: String,
        /// The wrapped cause.
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Wrap this error with the operation that was being attempted.
    #[must_use]
    pub fn context(self, context: impl fmt::Display) -> Self {
        Self::Context {
            context: context.to_string(),
            source: Box::new(self),
        }
    }
}

/// Add `.context()` to any `Result<T, E>` whose error converts into [`Error`].
pub trait ResultExt<T> {
    /// Convert the error into the project error and wrap it with `context`.
    ///
    /// # Errors
    /// Returns the wrapped project error when the receiver is `Err`.
    fn context(self, context: impl fmt::Display) -> Result<T>;

    /// Like [`ResultExt::context`] but the context is computed lazily, so the
    /// (possibly expensive) message is only built on the error path.
    ///
    /// # Errors
    /// Returns the wrapped project error when the receiver is `Err`.
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn context(self, context: impl fmt::Display) -> Result<T> {
        self.map_err(|e| e.into().context(context))
    }

    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.into().context(context()))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::{Error, ResultExt};

    #[test]
    fn context_chains_the_source() {
        let base = Error::Store("disk full".into());
        let wrapped = base.context("persisting record");
        assert_eq!(wrapped.to_string(), "persisting record");
        let source = wrapped.source().expect("a chained source");
        assert_eq!(source.to_string(), "store error: disk full");
    }

    #[test]
    fn result_ext_wraps_into_project_error() {
        let res: Result<(), Error> = Err(Error::Config("missing ns".into()));
        let wrapped = res.context("loading runtime config").unwrap_err();
        assert_eq!(wrapped.to_string(), "loading runtime config");
    }

    #[test]
    fn with_context_defers_message_construction() {
        let res: Result<(), Error> = Err(Error::Store("timeout".into()));
        let wrapped = res.with_context(|| format!("write to {}", "records")).unwrap_err();
        assert_eq!(wrapped.to_string(), "write to records");
    }
}
