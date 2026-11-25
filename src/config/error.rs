//! `ConfigError` and its Display/Error plumbing.

use std::path::{Path, PathBuf};

use crate::suggestions::Suggestion;

// ── ConfigError ───────────────────────────────────────────────────────

/// Unified error type for config operations.
#[derive(Debug)]
pub enum ConfigError {
    /// Config file not found at the expected path.
    MissingFile {
        path: PathBuf,
        tool: String,
        suggestion: Suggestion,
    },
    /// Config file could not be parsed (e.g., invalid TOML).
    ParseError {
        path: PathBuf,
        detail: String,
        suggestion: Suggestion,
    },
    /// Config content failed validation.
    ValidationError {
        path: PathBuf,
        detail: String,
        suggestion: Suggestion,
    },
    /// Type mismatch when downcasting a config.
    TypeMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    /// Underlying IO error.
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl ConfigError {
    /// Create a `MissingFile` error with a self-healing suggestion.
    pub fn missing_file(path: PathBuf, tool: &str) -> Self {
        Self::MissingFile {
            path,
            tool: tool.to_string(),
            suggestion: Suggestion::Fix {
                description: format!("Config file not found. Run `{} init` to create one", tool),
                command: Some(format!("{} init", tool)),
            },
        }
    }

    /// Create a `ParseError` with a self-healing suggestion.
    pub fn parse_error(path: PathBuf, detail: impl Into<String>) -> Self {
        Self::ParseError {
            path,
            detail: detail.into(),
            suggestion: Suggestion::Fix {
                description: "Config file is malformed. Run `doctor` to diagnose and fix"
                    .to_string(),
                command: None, // tool-specific; tool wraps this
            },
        }
    }

    /// Create a `ValidationError` with a self-healing suggestion.
    pub fn validation_error(path: PathBuf, detail: impl Into<String>) -> Self {
        Self::ValidationError {
            path,
            detail: detail.into(),
            suggestion: Suggestion::Fix {
                description: "Config validation failed. Run `doctor --fix` to resolve".to_string(),
                command: Some("doctor --fix".to_string()),
            },
        }
    }

    /// Create a `TypeMismatch` error.
    pub fn type_mismatch(
        path: PathBuf,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::TypeMismatch {
            path,
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Get the path associated with this error.
    pub fn path(&self) -> &Path {
        match self {
            ConfigError::MissingFile { path, .. } => path,
            ConfigError::ParseError { path, .. } => path,
            ConfigError::ValidationError { path, .. } => path,
            ConfigError::TypeMismatch { path, .. } => path,
            ConfigError::IoError { path, .. } => path,
        }
    }

    /// Get the self-healing suggestion for this error.
    pub fn to_suggestion(&self) -> Suggestion {
        match self {
            ConfigError::MissingFile { suggestion, .. }
            | ConfigError::ParseError { suggestion, .. }
            | ConfigError::ValidationError { suggestion, .. } => suggestion.clone(),
            ConfigError::TypeMismatch { .. } => Suggestion::Fix {
                description: "Internal error: config type mismatch.".to_string(),
                command: None,
            },
            ConfigError::IoError { source, .. } => Suggestion::Fix {
                description: format!("IO error: {}", source),
                command: None,
            },
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingFile { path, tool, .. } => fmt_missing_file(f, path, tool),
            ConfigError::ParseError { path, detail, .. } => {
                fmt_detail(f, "config parse error", path, detail)
            }
            ConfigError::ValidationError { path, detail, .. } => {
                fmt_detail(f, "config validation error", path, detail)
            }
            ConfigError::TypeMismatch {
                path,
                expected,
                actual,
            } => fmt_type_mismatch(f, path, expected, actual),
            ConfigError::IoError { path, source } => {
                write!(f, "IO error for {}: {}", path.display(), source)
            }
        }
    }
}

/// `"config file not found: <path> (tool: <tool>)"`.
fn fmt_missing_file(f: &mut std::fmt::Formatter<'_>, path: &Path, tool: &str) -> std::fmt::Result {
    write!(
        f,
        "config file not found: {} (tool: {})",
        path.display(),
        tool
    )
}

/// `"<prefix>: <path>: <detail>"` — shape shared by parse and validation
/// errors.
fn fmt_detail(
    f: &mut std::fmt::Formatter<'_>,
    prefix: &str,
    path: &Path,
    detail: &str,
) -> std::fmt::Result {
    write!(f, "{}: {}: {}", prefix, path.display(), detail)
}

/// Human message for a type mismatch.
fn fmt_type_mismatch(
    f: &mut std::fmt::Formatter<'_>,
    path: &Path,
    expected: &str,
    actual: &str,
) -> std::fmt::Result {
    write!(
        f,
        "config type mismatch for {}: expected {}, got {}",
        path.display(),
        expected,
        actual
    )
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(source: std::io::Error) -> Self {
        let path = PathBuf::from("<unknown>");
        Self::IoError { path, source }
    }
}
