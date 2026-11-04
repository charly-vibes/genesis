//! Unit tests for `config` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use serde::Deserialize;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────

fn tmp() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// A mock config struct for testing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct MockConfig {
    name: String,
    version: u32,
    enabled: bool,
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

/// A config that validates its fields.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct ValidatedConfig {
    name: String,
    timeout: u64,
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

// ── ConfigStore: task 4.1–4.6 ─────────────────────────────────────

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
                Ok(vec![ConfigValidation::error(
                    "name",
                    "name must not be empty",
                )])
            } else {
                Ok(vec![])
            }
        }
    }

    reg.register::<StrictConfig>("strict", "strict.toml");

    // Write a config with an empty name — validation should catch it
    let config = StrictConfig {
        name: "".to_string(),
    };
    config.write(dir.path()).require("value");

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

    // 2. Register a second tool
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

    reg.register::<OtherConfig>("other", "other.toml");

    // 3. Write configs
    let mock = MockConfig {
        name: "lifecycle".to_string(),
        version: 3,
        enabled: true,
    };
    mock.write(dir.path()).require("value");

    let other = OtherConfig {
        host: "example.com".to_string(),
        port: 443,
    };
    other.write(dir.path()).require("value");

    // 4. Create store and discover
    let store = ConfigStore::new(reg);
    let discovered = ConfigStore::discover(dir.path(), store.registry());
    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().all(|d| d.found));

    // 5. Read typed configs
    let read_mock: MockConfig = store.get("mock", dir.path()).require("value");
    assert_eq!(read_mock.name, "lifecycle");

    let read_other: OtherConfig = store.get("other", dir.path()).require("value");
    assert_eq!(read_other.host, "example.com");
    assert_eq!(read_other.port, 443);

    // 6. Generate managed block
    let block = store.managed_block(dir.path());
    assert!(block.contains("✅ found"));
    assert!(block.contains("mock"));
    assert!(block.contains("other"));
}
