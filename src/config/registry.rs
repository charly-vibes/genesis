//! Type-erased config registry: registration, discovery, and validation
//! across registered tools.

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::suggestions::Suggestion;

use super::error::ConfigError;
use super::validation::{ConfigFile, ConfigValidation};

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
pub struct ConfigEntry {
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
        let entry = ConfigEntry {
            tool_name,
            marker,
            factory: load_factory::<T>(),
            validator: validate_factory::<T>(),
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

/// Factory closure: read `T` from its path under `repo_root`.
fn load_factory<T: ConfigFile + DeserializeOwned + 'static>() -> ConfigFactory {
    Box::new(move |repo_root| {
        let path = T::path(repo_root);
        let config = T::read_from(&path)?;
        Ok(Box::new(config) as Box<dyn Any>)
    })
}

/// Validator closure: validate a stored `T`.
fn validate_factory<T: ConfigFile + DeserializeOwned + 'static>() -> ConfigValidator {
    Box::new(move |any| {
        let config = any
            .downcast_ref::<T>()
            .ok_or_else(|| ConfigError::TypeMismatch {
                path: PathBuf::from("<internal>"),
                expected: std::any::type_name::<T>().to_string(),
                actual: "<unknown>".to_string(),
            })?;
        config.validate()
    })
}
