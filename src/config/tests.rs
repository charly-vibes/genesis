//! Unit tests for `config` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

/// A mock config struct for testing.
/// A config that validates its fields.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(super) struct MockConfig {
    pub(super) name: String,
    pub(super) version: u32,
    pub(super) enabled: bool,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            name: "mock".to_string(),
            version: 1,
            enabled: true,
        }
    }
}

impl ConfigFile for MockConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".mock/config.toml")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub(super) struct ValidatedConfig {
    pub(super) name: String,
    pub(super) timeout: u64,
}

impl Default for ValidatedConfig {
    fn default() -> Self {
        Self {
            name: "app".to_string(),
            timeout: 30,
        }
    }
}

impl ConfigFile for ValidatedConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join("app.toml")
    }

    fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError> {
        let mut results = Vec::new();
        if self.name.is_empty() {
            results.push(ConfigValidation::error("name", "name must not be empty"));
        }
        if self.timeout == 0 {
            results.push(ConfigValidation::error("timeout", "timeout must be > 0"));
        } else if self.timeout > 300 {
            results.push(ConfigValidation::warning(
                "timeout",
                "timeout > 300s may cause issues",
            ));
        }
        Ok(results)
    }
}
// ── ConfigError: task 1.1–1.4 ─────────────────────────────────────
#[test]
fn test_missing_file_error_contains_suggestion() {
    let err = ConfigError::missing_file(PathBuf::from("test.toml"), "testtool");
    let suggestion = err.to_suggestion();
    let msg = suggestion.message();
    assert!(msg.contains("testtool init"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_parse_error_contains_suggestion() {
    let err = ConfigError::parse_error(PathBuf::from("bad.toml"), "invalid syntax");
    let suggestion = err.to_suggestion();
    let msg = suggestion.message();
    assert!(msg.contains("doctor"));
    assert!(msg.contains("malformed"));
}

#[test]
fn test_validation_error_contains_suggestion() {
    let err = ConfigError::validation_error(PathBuf::from("bad.toml"), "field x is wrong");
    let suggestion = err.to_suggestion();
    let msg = suggestion.message();
    assert!(msg.contains("doctor --fix"));
}

#[test]
fn test_config_error_display() {
    let err = ConfigError::missing_file(PathBuf::from("test.toml"), "tool");
    let display = format!("{}", err);
    assert!(display.contains("test.toml"));
    assert!(display.contains("tool"));
}

#[test]
fn test_config_error_path() {
    let err = ConfigError::missing_file(PathBuf::from("test.toml"), "tool");
    assert_eq!(err.path(), Path::new("test.toml"));
}

#[test]
fn test_io_error_from_std() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let config_err: ConfigError = io_err.into();
    match config_err {
        ConfigError::IoError { .. } => {} // expected
        _ => panic!("expected IoError variant"),
    }
}

#[test]
fn test_type_mismatch_error_contains_path() {
    let err = ConfigError::type_mismatch(
        PathBuf::from("config.toml"),
        "MockConfig",
        "ValidatedConfig",
    );
    let display = format!("{}", err);
    assert!(display.contains("config.toml"));
    assert!(display.contains("MockConfig"));
    assert!(display.contains("ValidatedConfig"));
}

#[test]
fn test_type_mismatch_to_suggestion() {
    let err = ConfigError::type_mismatch(PathBuf::from("config.toml"), "expected", "actual");
    let suggestion = err.to_suggestion();
    let msg = suggestion.message();
    assert!(msg.contains("type mismatch"));
    assert!(suggestion.footer().is_none());
}

// ── ConfigValidation: task 2.3 ────────────────────────────────────

#[test]
fn test_validation_warning() {
    let v = ConfigValidation::warning("timeout", "value is high");
    assert_eq!(v.severity, ValidationSeverity::Warning);
    assert_eq!(v.field, "timeout");
}

#[test]
fn test_validation_error() {
    let v = ConfigValidation::error("name", "must not be empty");
    assert_eq!(v.severity, ValidationSeverity::Error);
}

// ── ConfigFile: task 2.1–2.3 ──────────────────────────────────────

#[test]
fn test_config_file_path() {
    let dir = tmp();
    let path = MockConfig::path(dir.path());
    assert_eq!(path, dir.path().join(".mock/config.toml"));
}

#[test]
fn test_config_file_read_missing() {
    let dir = tmp();
    let result = MockConfig::read(dir.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::MissingFile { .. } => {} // expected
        e => panic!("expected MissingFile, got: {:?}", e),
    }
}

#[test]
fn test_config_file_read_write_roundtrip() {
    let dir = tmp();
    let config = MockConfig {
        name: "test".to_string(),
        version: 42,
        enabled: false,
    };

    // Write
    config.write(dir.path()).require("value");

    // Read back
    let read = MockConfig::read(dir.path()).require("value");
    assert_eq!(read, config);
}

#[test]
fn test_config_file_read_parse_error() {
    let dir = tmp();
    let path = MockConfig::path(dir.path());
    std::fs::create_dir_all(path.parent().require("value")).require("value");
    std::fs::write(&path, "not valid toml {{{").require("value");

    let result = MockConfig::read(dir.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::ParseError { .. } => {} // expected
        e => panic!("expected ParseError, got: {:?}", e),
    }
}

#[test]
fn test_config_file_validate_default_noop() {
    // MockConfig doesn't override validate, so it should return empty
    let config = MockConfig::default();
    let results = config.validate().require("value");
    assert!(results.is_empty());
}

#[test]
fn test_config_file_validate_custom() {
    let config = ValidatedConfig {
        name: "".to_string(),
        timeout: 0,
    };
    let results = config.validate().require("value");
    assert!(!results.is_empty());
    assert!(results.iter().any(|v| v.field == "name"));
    assert!(results.iter().any(|v| v.field == "timeout"));
}

#[test]
fn test_config_file_validate_warning() {
    let config = ValidatedConfig {
        name: "app".to_string(),
        timeout: 500,
    };
    let results = config.validate().require("value");
    assert!(
        results
            .iter()
            .any(|v| v.field == "timeout" && v.severity == ValidationSeverity::Warning)
    );
}

// ── ConfigRegistry: task 3.1–3.5 ──────────────────────────────────

#[test]
fn test_registry_empty_by_default() {
    let reg = ConfigRegistry::new();
    assert!(reg.is_empty());
    assert!(reg.registered_tools().is_empty());
}

#[test]
fn test_registry_register_and_check() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    assert!(reg.is_registered("mock"));
    assert_eq!(reg.len(), 1);
}

#[test]
fn test_registry_register_then_unregister() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    assert!(reg.unregister("mock"));
    assert!(!reg.is_registered("mock"));
    assert!(reg.is_empty());
}

#[test]
fn test_registry_register_multiple() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");
    assert_eq!(reg.len(), 2);
    let tools = reg.registered_tools();
    assert!(tools.contains(&"mock"));
    assert!(tools.contains(&"app"));
}

#[test]
#[should_panic(expected = "already registered")]
fn test_registry_register_duplicate_panics() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    assert!(
        reg.is_registered("mock"),
        "precondition: first registration must succeed"
    );
    reg.register::<MockConfig>("mock", ".mock/config.toml");
}

#[test]
fn test_registry_marker() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    assert_eq!(reg.marker("mock"), Some(".mock/config.toml"));
    assert_eq!(reg.marker("unknown"), None);
}

#[test]
fn test_registry_get_typed() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");

    // Write a config first
    let config = MockConfig {
        name: "test".to_string(),
        version: 99,
        enabled: true,
    };
    config.write(dir.path()).require("value");

    // Read back through registry
    let any = reg.get("mock", dir.path()).require("value");
    let downcast = any.downcast::<MockConfig>().require("value");
    assert_eq!(downcast.name, "test");
    assert_eq!(downcast.version, 99);
}

#[test]
fn test_registry_get_unknown_tool() {
    let reg = ConfigRegistry::new();
    let dir = tmp();
    let result = reg.get("unknown", dir.path());
    assert!(result.is_err());
}

#[test]
fn test_registry_iter() {
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<ValidatedConfig>("app", "app.toml");

    let entries: Vec<(&str, &str)> = reg.iter().collect();
    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&("mock", ".mock/config.toml")));
    assert!(entries.contains(&("app", "app.toml")));
}
