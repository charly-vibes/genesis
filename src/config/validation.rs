//! Validation results and the [`ConfigFile`] trait every tool config
//! implements.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::suggestions::Suggestion;

use super::error::ConfigError;

// ── ConfigValidation ──────────────────────────────────────────────────

/// A single validation result for a config field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidation {
    /// The field that failed validation (e.g., "timeout").
    pub field: String,
    /// Human-readable description of the issue.
    pub message: String,
    /// Severity level.
    pub severity: ValidationSeverity,
}

/// Severity of a validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// The config can still be used with a reasonable default.
    Warning,
    /// The config cannot be used as-is.
    Error,
}

impl ConfigValidation {
    /// Create a warning-level validation result.
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Warning,
        }
    }

    /// Create an error-level validation result.
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: ValidationSeverity::Error,
        }
    }
}

// ── ConfigFile trait ──────────────────────────────────────────────────

/// Trait for tool-specific config files.
///
/// Each tool implements this trait for its config struct. Genesis provides
/// the read/write/validate machinery through the blanket impl.
///
/// # Required
///
/// - `fn path(repo_root: &Path) -> PathBuf` — where the config file lives.
///
/// # Provided
///
/// - `fn read(...)` — deserializes from the file at `path()`.
/// - `fn write(...)` — serializes to the file at `path()`.
/// - `fn validate(...)` — no-op by default; override for domain checks.
pub trait ConfigFile: Sized {
    /// Return the path to this config file relative to `repo_root`.
    fn path(repo_root: &Path) -> PathBuf;

    /// Read and parse the config from its file.
    fn read(repo_root: &Path) -> Result<Self, ConfigError>
    where
        Self: DeserializeOwned,
    {
        let path = Self::path(repo_root);
        Self::read_from(&path)
    }

    /// Write the config to its file.
    fn write(&self, repo_root: &Path) -> Result<(), ConfigError>
    where
        Self: Serialize,
    {
        let path = Self::path(repo_root);
        self.write_to(&path)
    }

    /// Validate the config after reading.
    ///
    /// Returns `Ok(())` by default. Override for domain-specific checks.
    fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError> {
        let _ = self;
        Ok(Vec::new())
    }

    // ── Internal helpers (used by default impls) ───────────────────────

    /// Read from an explicit path (used by ConfigStore).
    fn read_from(path: &Path) -> Result<Self, ConfigError>
    where
        Self: DeserializeOwned,
    {
        if !path.exists() {
            return Err(ConfigError::missing_file(
                path.to_path_buf(),
                // Infer tool name from type name as fallback
                std::any::type_name::<Self>()
                    .rsplit("::")
                    .next()
                    .unwrap_or("tool"),
            ));
        }
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;
        if content.trim().is_empty() {
            return Err(ConfigError::parse_error(
                path.to_path_buf(),
                "config file is empty",
            ));
        }
        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::parse_error(path.to_path_buf(), e.to_string()))?;
        Ok(config)
    }

    /// Write to an explicit path (used by ConfigStore).
    fn write_to(&self, path: &Path) -> Result<(), ConfigError>
    where
        Self: Serialize,
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::IoError {
                path: path.to_path_buf(),
                source: e,
            })?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            detail: e.to_string(),
            suggestion: Suggestion::Fix {
                description: "Failed to serialize config.".to_string(),
                command: None,
            },
        })?;
        std::fs::write(path, content).map_err(|e| ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    }
}

// Default `read()` and `write()` impls on the `ConfigFile` trait provide
// the serde-based parsing/serialization for any `T: DeserializeOwned + Serialize`.
// Tools still must implement `path()` — it can't be provided generically.
// No blanket `impl<T: ...> ConfigFile for T` is needed.
