//! ConfigStore + registry integration tests (split from tests.rs for the
//! pretender file-lines budget).

use super::tests::{MockConfig, ValidatedConfig};
use super::*;
use crate::testutil::TestUnwrap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Fresh temp dir for filesystem tests.
fn tmp() -> TempDir {
    TempDir::new().expect("tempdir")
}

#[path = "tests_store_lifecycle.rs"]
mod lifecycle;

/// Registry with both standard fixtures (`mock`, `app`) registered.
fn two_tool_registry() -> ConfigRegistry {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");
    reg
}

/// Whether `tool_name` appears among `discovered` as found.
fn is_found(discovered: &[DiscoveredConfig], tool_name: &str) -> bool {
    discovered
        .iter()
        .find(|d| d.tool_name == tool_name)
        .require("value")
        .found
}

// ── ConfigStore: task 4.1–4.6 ─────────────────────────────────────

/// A config whose validation rejects empty names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct StrictConfig {
    name: String,
}

impl Default for StrictConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

impl ConfigFile for StrictConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join("strict.toml")
    }

    fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError> {
        if self.name.is_empty() {
            return Ok(vec![ConfigValidation::error(
                "name",
                "name must not be empty",
            )]);
        }
        Ok(Vec::new())
    }
}

#[test]
fn test_config_store_new() {
    let reg = ConfigRegistry::new();
    let store = ConfigStore::new(reg);
    assert!(store.registry().is_empty());
}

#[test]
fn test_validate_all_actually_validates() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<StrictConfig>("strict", "strict.toml");

    // Write a config with an empty name — validation should catch it
    StrictConfig {
        name: String::new(),
    }
    .write(dir.path())
    .require("write strict.toml");

    let store = ConfigStore::new(reg);
    let results = store.validate_all(dir.path());
    assert!(
        !results.is_empty(),
        "validate_all should find validation errors"
    );
    assert!(results.iter().any(|r| r.field == "name"));
}

#[test]
fn test_validate_all_skips_missing_files() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");

    // No config written — validate_all should not error on missing
    let store = ConfigStore::new(reg);
    let results = store.validate_all(dir.path());
    let errors: Vec<_> = results
        .iter()
        .filter(|r| r.severity == ValidationSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "missing files should not produce validation errors"
    );
}

#[test]
fn test_config_store_discover_all_found() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");

    // Create both config files
    let mock = MockConfig::default();
    mock.write(dir.path()).require("value");
    let app = ValidatedConfig::default();
    app.write(dir.path()).require("value");

    let discovered = ConfigStore::discover(dir.path(), &reg);
    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().all(|d| d.found));
}

#[test]
fn test_config_store_discover_some_missing() {
    let dir = tmp();
    let reg = two_tool_registry();

    // Only create one config
    MockConfig::default().write(dir.path()).require("value");

    let discovered = ConfigStore::discover(dir.path(), &reg);
    assert_eq!(discovered.len(), 2);
    assert!(is_found(&discovered, "mock"));
    assert!(!is_found(&discovered, "app"));
}

#[test]
fn test_config_store_get_typed() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");

    let config = MockConfig {
        name: "typed".to_string(),
        version: 1,
        enabled: true,
    };
    config.write(dir.path()).require("value");

    let store = ConfigStore::new(reg);
    let result: MockConfig = store.get("mock", dir.path()).require("value");
    assert_eq!(result.name, "typed");
}

#[test]
fn test_config_store_get_type_mismatch() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");

    let config = MockConfig::default();
    config.write(dir.path()).require("value");

    let store = ConfigStore::new(reg);
    // Try to get as ValidatedConfig — should fail with TypeMismatch
    let result: Result<ValidatedConfig, ConfigError> = store.get("mock", dir.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::TypeMismatch { .. } => {} // expected
        e => panic!("expected TypeMismatch, got: {:?}", e),
    }
}

#[test]
fn test_config_store_managed_block() {
    let dir = tmp();
    let reg = two_tool_registry();
    MockConfig::default().write(dir.path()).require("value");

    let store = ConfigStore::new(reg);
    let block = store.managed_block(dir.path());

    // Block should mention both tools
    assert!(block.contains("mock"));
    assert!(block.contains("app"));
    assert!(block.contains("✅ found"));
    assert!(block.contains("❌ missing"));
}

#[test]
fn test_config_store_managed_block_all_found() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");

    let config = MockConfig::default();
    config.write(dir.path()).require("value");

    let store = ConfigStore::new(reg);
    let block = store.managed_block(dir.path());
    assert!(block.contains("✅ found"));
    assert!(!block.contains("❌"));
}

#[test]
fn test_read_empty_file_returns_parse_error() {
    let dir = tmp();
    let path = MockConfig::path(dir.path());
    std::fs::create_dir_all(path.parent().require("value")).require("value");
    std::fs::write(&path, "").require("value");

    let result = MockConfig::read(dir.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::ParseError { detail, .. } => {
            assert!(
                detail.contains("empty"),
                "expected empty file message, got: {}",
                detail
            );
        }
        e => panic!("expected ParseError for empty file, got: {:?}", e),
    }
}

#[test]
fn test_read_whitespace_only_file_returns_parse_error() {
    let dir = tmp();
    let path = MockConfig::path(dir.path());
    std::fs::create_dir_all(path.parent().require("value")).require("value");
    std::fs::write(&path, "   \n\n  ").require("value");

    let result = MockConfig::read(dir.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::ParseError { detail, .. } => {
            assert!(
                detail.contains("empty"),
                "expected empty file message, got: {}",
                detail
            );
        }
        e => panic!("expected ParseError for whitespace-only file, got: {:?}", e),
    }
}
