//! `ConfigStore`: discovery-backed view over a [`ConfigRegistry`], plus the
//! managed-block rendering for status dashboards.

use std::path::{Path, PathBuf};

use super::error::ConfigError;
use super::registry::ConfigRegistry;
use super::validation::{ConfigFile, ConfigValidation};

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

        lines.extend(discovered.iter().map(config_row));
        lines.extend(missing_footer(&discovered));

        lines.join("\n")
    }
}

/// One markdown table row for a discovered config.
fn config_row(dc: &DiscoveredConfig) -> String {
    let status = if dc.found {
        "✅ found".to_string()
    } else {
        format!("❌ missing — run `{}` init", dc.tool_name)
    };
    format!("| {} | {} | {}", dc.tool_name, dc.marker, status)
}

/// Footer naming the missing configs, or empty when everything is found.
fn missing_footer(discovered: &[DiscoveredConfig]) -> Vec<String> {
    let missing: Vec<&DiscoveredConfig> = discovered.iter().filter(|d| !d.found).collect();
    if missing.is_empty() {
        return Vec::new();
    }
    let tool_names: Vec<&str> = missing.iter().map(|d| d.tool_name.as_str()).collect();
    vec![
        String::new(),
        format!(
            "Missing configs: {}. Run `doctor` on the respective tools to fix issues.",
            tool_names.join(", ")
        ),
    ]
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
