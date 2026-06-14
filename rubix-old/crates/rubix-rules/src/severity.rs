//! Local mirror of the rubix severity set.
//!
//! Deliberately defined here rather than imported from `rubix-core`: this crate
//! is standalone (the integrating session maps this type onto the canonical
//! severity). The wire form is the lowercase string a finding carries
//! (`info` / `warning` / `fault`), matching the design doc's `severity` field.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::RuleError;

/// A finding's severity. Mirrors the rubix `info` / `warning` / `fault` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational; not a problem on its own.
    Info,
    /// A condition worth attention but not a failure.
    Warning,
    /// A failure condition.
    Fault,
}

impl Severity {
    /// The lowercase wire string for this severity.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Fault => "fault",
        }
    }

    /// Parse a severity from its wire string.
    ///
    /// A script writes `finding("fault", …)`; an unknown string is a runtime
    /// error rather than a silent coercion, so a typo never downgrades a fault.
    pub fn parse(s: &str) -> Result<Self, RuleError> {
        match s {
            "info" => Ok(Severity::Info),
            "warning" => Ok(Severity::Warning),
            "fault" => Ok(Severity::Fault),
            other => Err(RuleError::Runtime(format!(
                "unknown severity `{other}` (expected info, warning, or fault)"
            ))),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
