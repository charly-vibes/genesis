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
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");

    // Only create one config
    let mock = MockConfig::default();
    mock.write(dir.path()).require("value");

    let discovered = ConfigStore::discover(dir.path(), &reg);
    assert_eq!(discovered.len(), 2);

    let mock_dc = discovered
        .iter()
        .find(|d| d.tool_name == "mock")
        .require("value");
    assert!(mock_dc.found);

    let app_dc = discovered
        .iter()
        .find(|d| d.tool_name == "app")
        .require("value");
    assert!(!app_dc.found);
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
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");

    // Only create mock config
    let config = MockConfig::default();
    config.write(dir.path()).require("value");

    let store = ConfigStore::new(reg);
    let block = store.managed_block(dir.path());

    // Block should mention both tools
    assert!(block.contains("mock"));
    assert!(block.contains("app"));
    // Mock should be found
    assert!(block.contains("✅ found"));
    // App should be missing
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

// ── Integration: full lifecycle ───────────────────────────────────

#[test]
fn test_integration_full_lifecycle() {
    let dir = tmp();

    // 1. Create registry with two tools
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<OtherConfig>("other", "other.toml");

    // 2. Write configs
    write_lifecycle_configs(dir.path());

    // 3. Create store and discover
    let store = ConfigStore::new(reg);
    let discovered = ConfigStore::discover(dir.path(), store.registry());
    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().all(|d| d.found));

    // 4. Read typed configs
    let read_mock: MockConfig = store.get("mock", dir.path()).require("value");
    let read_other: OtherConfig = store.get("other", dir.path()).require("value");
    assert_round_trip(&read_mock, "lifecycle", &read_other, "example.com");

    // 5. Generate managed block
    let block = store.managed_block(dir.path());
    assert!(block.contains("✅ found"));
    assert!(block.contains("mock"));
    assert!(block.contains("other"));
}

/// Both lifecycle round-trips: names must survive the write/read cycle.
fn assert_round_trip(
    mock: &MockConfig,
    expected_mock_name: &str,
    other: &OtherConfig,
    expected_host: &str,
) {
    assert_eq!(mock.name, expected_mock_name);
    assert_eq!(other.host, expected_host);
    assert_eq!(other.port, 443);
}

/// The second lifecycle tool: an `other.toml` with host/port.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct OtherConfig {
    host: String,
    port: u16,
}

impl Default for OtherConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

impl ConfigFile for OtherConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join("other.toml")
    }
}

/// Write both lifecycle configs under `root`.
fn write_lifecycle_configs(root: &Path) {
    MockConfig {
        name: "lifecycle".to_string(),
        version: 3,
        enabled: true,
    }
    .write(root)
    .require("write mock config");

    OtherConfig {
        host: "example.com".to_string(),
        port: 443,
    }
    .write(root)
    .require("write other config");
}
