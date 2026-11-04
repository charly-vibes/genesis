//! Shared config management for suite tools.
//!
//! Provides `ConfigFile` trait, `ConfigRegistry`, `ConfigStore` — each tool
//! just implements the trait and gets read/write/discover/validate for free.
//!
//! ## AIX: self-healing config errors
//!
//! Every config error carries a `Suggestion` footer so the tool's error-sink
//! wiring (the `→ Run: …` pattern) works without changes.

use crate::suggestions::Suggestion;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
            ConfigError::MissingFile { path, tool, .. } => {
                write!(
                    f,
                    "config file not found: {} (tool: {})",
                    path.display(),
                    tool
                )
            }
            ConfigError::ParseError { path, detail, .. } => {
                write!(f, "config parse error: {}: {}", path.display(), detail)
            }
            ConfigError::ValidationError { path, detail, .. } => {
                write!(f, "config validation error: {}: {}", path.display(), detail)
            }
            ConfigError::TypeMismatch {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "config type mismatch for {}: expected {}, got {}",
                    path.display(),
                    expected,
                    actual
                )
            }
            ConfigError::IoError { path, source } => {
                write!(f, "IO error for {}: {}", path.display(), source)
            }
        }
    }
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

// ── ConfigRegistry ────────────────────────────────────────────────────

/// A type-erased factory for config files.
///
/// Each registered tool stores a factory function and its marker path.
/// The factory takes a repo root and returns a boxed config.
type ConfigFactory = Box<dyn Fn(&Path) -> Result<Box<dyn Any>, ConfigError> + Send + Sync>;

/// A type-erased validator for config files.
///
/// Takes a `&dyn Any` (the config) and returns validation results.
type ConfigValidator =
    Box<dyn Fn(&dyn Any) -> Result<Vec<ConfigValidation>, ConfigError> + Send + Sync>;

/// A tool's entry in the registry.
struct ConfigEntry {
    /// Display name of the tool.
    tool_name: &'static str,
    /// Marker path relative to repo root (e.g., `.wai/config.toml`).
    marker: &'static str,
    /// Factory function that reads and parses the config.
    factory: ConfigFactory,
    /// Validator function that validates a parsed config.
    validator: ConfigValidator,
}

/// Runtime registry of tool configs.
///
/// Tools register their config type at startup. The registry is thread-safe
/// and can be used for discovery, validation, and typed access.
#[derive(Default)]
pub struct ConfigRegistry {
    entries: HashMap<&'static str, ConfigEntry>,
}

impl ConfigRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a config type for a tool.
    ///
    /// `marker` is the relative path from repo root (e.g., `".wai/config.toml"`).
    ///
    /// # Panics
    ///
    /// Panics if `tool_name` is already registered (call `unregister` first).
    pub fn register<T: ConfigFile + DeserializeOwned + 'static>(
        &mut self,
        tool_name: &'static str,
        marker: &'static str,
    ) {
        assert!(
            !self.entries.contains_key(tool_name),
            "tool '{}' is already registered",
            tool_name
        );
        let factory: ConfigFactory = Box::new(move |repo_root| {
            let path = T::path(repo_root);
            let config = T::read_from(&path)?;
            Ok(Box::new(config) as Box<dyn Any>)
        });
        let validator: ConfigValidator = Box::new(move |any| {
            let config = any
                .downcast_ref::<T>()
                .ok_or_else(|| ConfigError::TypeMismatch {
                    path: PathBuf::from("<internal>"),
                    expected: std::any::type_name::<T>().to_string(),
                    actual: "<unknown>".to_string(),
                })?;
            config.validate()
        });
        let entry = ConfigEntry {
            tool_name,
            marker,
            factory,
            validator,
        };
        self.entries.insert(tool_name, entry);
    }

    /// Unregister a tool's config.
    pub fn unregister(&mut self, tool_name: &str) -> bool {
        self.entries.remove(tool_name).is_some()
    }

    /// Check if a tool is registered.
    pub fn is_registered(&self, tool_name: &str) -> bool {
        self.entries.contains_key(tool_name)
    }

    /// Get the marker path for a registered tool.
    pub fn marker(&self, tool_name: &str) -> Option<&'static str> {
        self.entries.get(tool_name).map(|e| e.marker)
    }

    /// List all registered tool names.
    pub fn registered_tools(&self) -> Vec<&'static str> {
        self.entries.keys().copied().collect()
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read and parse a config for a registered tool.
    ///
    /// Returns the config as a `Box<dyn Any>`. Use `ConfigStore::get` for
    /// typed access.
    pub fn get(&self, tool_name: &str, repo_root: &Path) -> Result<Box<dyn Any>, ConfigError> {
        let entry = self.entries.get(tool_name).ok_or_else(|| {
            let path = PathBuf::from(tool_name);
            ConfigError::MissingFile {
                path,
                tool: tool_name.to_string(),
                suggestion: Suggestion::Fix {
                    description: format!(
                        "Tool '{}' is not registered in the config registry",
                        tool_name
                    ),
                    command: None,
                },
            }
        })?;
        (entry.factory)(repo_root)
    }

    /// Iterate over all registered entries.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.values().map(|e| (e.tool_name, e.marker))
    }

    /// Validate all registered configs in a repo root.
    ///
    /// Reads each registered config and calls its `validate()` method.
    /// Missing files are silently skipped (not treated as validation errors).
    pub fn validate_registered(&self, repo_root: &Path) -> Vec<ConfigValidation> {
        let mut results = Vec::new();
        for (tool_name, _marker) in self.iter() {
            match self.get(tool_name, repo_root) {
                Ok(any) => {
                    let entry = &self.entries[tool_name];
                    match (entry.validator)(&*any) {
                        Ok(validations) => results.extend(validations),
                        Err(e) => results.push(ConfigValidation::error(
                            tool_name,
                            format!("Validation failed: {}", e),
                        )),
                    }
                }
                Err(ConfigError::MissingFile { .. }) => {
                    // Missing file is not a validation error
                }
                Err(e) => {
                    results.push(ConfigValidation::error(
                        tool_name,
                        format!("Failed to read config: {}", e),
                    ));
                }
            }
        }
        results
    }
}

// ── ConfigStore ───────────────────────────────────────────────────────

/// Unified access to all registered tool configs.
///
/// Wraps a `ConfigRegistry` and provides discovery, validation, and
/// managed-block generation.
pub struct ConfigStore {
    registry: ConfigRegistry,
}

impl ConfigStore {
    /// Create a new config store wrapping the given registry.
    pub fn new(registry: ConfigRegistry) -> Self {
        Self { registry }
    }

    /// Get a reference to the underlying registry.
    pub fn registry(&self) -> &ConfigRegistry {
        &self.registry
    }

    /// Discover all config files present in a repo root.
    ///
    /// Walks registered markers and returns `(tool_name, path, found)` for each.
    pub fn discover(repo_root: &Path, registry: &ConfigRegistry) -> Vec<DiscoveredConfig> {
        let mut results = Vec::new();
        for (tool_name, marker) in registry.iter() {
            let path = repo_root.join(marker);
            let found = path.exists();
            results.push(DiscoveredConfig {
                tool_name: tool_name.to_string(),
                marker: marker.to_string(),
                path: path.clone(),
                found,
            });
        }
        results
    }

    /// Validate all registered configs in a repo root.
    ///
    /// Reads each registered config and calls its `validate()` method.
    /// Missing files are silently skipped (not treated as validation errors).
    pub fn validate_all(&self, repo_root: &Path) -> Vec<ConfigValidation> {
        self.registry.validate_registered(repo_root)
    }

    /// Get a typed config for a registered tool.
    ///
    /// Returns `TypeMismatch` error if the config was registered with a different type.
    pub fn get<T: ConfigFile + 'static>(
        &self,
        tool_name: &str,
        repo_root: &Path,
    ) -> Result<T, ConfigError> {
        let any = self.registry.get(tool_name, repo_root)?;
        any.downcast::<T>().map(|boxed| *boxed).map_err(|_| {
            ConfigError::type_mismatch(
                T::path(repo_root),
                format!("expected '{}'", tool_name),
                std::any::type_name::<T>(),
            )
        })
    }

    /// Generate a managed-block content string showing config status.
    ///
    /// Returns a markdown table of (tool, path, status, next-step).
    pub fn managed_block(&self, repo_root: &Path) -> String {
        let discovered = Self::discover(repo_root, &self.registry);
        let mut lines = vec![
            "## Config files".to_string(),
            String::new(),
            "| Tool | Path | Status |".to_string(),
            "|------|------|--------|".to_string(),
        ];

        for dc in &discovered {
            let status = if dc.found {
                "✅ found".to_string()
            } else {
                format!("❌ missing — run `{} init`", dc.tool_name)
            };
            lines.push(format!("| {} | {} | {} |", dc.tool_name, dc.marker, status));
        }

        // Add footer with next steps for missing configs
        let missing: Vec<&DiscoveredConfig> = discovered.iter().filter(|d| !d.found).collect();
        if !missing.is_empty() {
            lines.push(String::new());
            let tool_names: Vec<&str> = missing.iter().map(|d| d.tool_name.as_str()).collect();
            lines.push(format!(
                "Missing configs: {}. Run `doctor` on the respective tools to fix issues.",
                tool_names.join(", ")
            ));
        }

        lines.join("\n")
    }
}

/// Result of a discovery operation.
#[derive(Debug, Clone)]
pub struct DiscoveredConfig {
    /// Tool name (e.g., "wai", "dont").
    pub tool_name: String,
    /// Marker path relative to repo root (e.g., ".wai/config.toml").
    pub marker: String,
    /// Absolute path to the config file.
    pub path: PathBuf,
    /// Whether the config file exists.
    pub found: bool,
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;
