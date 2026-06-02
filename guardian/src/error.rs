//! Error types for ratatui-guardian.
//!
//! All fallible operations return `Result<T, GuardianError>` rather than
//! silently dropping errors or panicking.

use std::fmt;
use std::path::PathBuf;

/// The top-level error type for all guardian operations.
#[derive(Debug)]
pub enum GuardianError {
    /// An I/O error occurred during persistence (save/load).
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// JSON serialization or deserialization failed.
    Json {
        context: String,
        source: serde_json::Error,
    },
    /// Attempted an operation when the profiler had no data.
    NoData {
        operation: String,
    },
    /// Invalid parameters were passed to a builder or configuration method.
    InvalidConfig {
        message: String,
    },
    /// A comparison cannot be performed (e.g. incompatible profiler states).
    ComparisonFailed {
        reason: String,
    },
}

impl fmt::Display for GuardianError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "I/O error on {}: {source}", path.display())
            }
            Self::Json { context, source } => {
                write!(f, "JSON error ({context}): {source}")
            }
            Self::NoData { operation } => {
                write!(f, "no data available for {operation}")
            }
            Self::InvalidConfig { message } => {
                write!(f, "invalid configuration: {message}")
            }
            Self::ComparisonFailed { reason } => {
                write!(f, "comparison failed: {reason}")
            }
        }
    }
}

impl std::error::Error for GuardianError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, GuardianError>;
