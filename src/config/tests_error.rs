//! `ConfigError` unit tests (split from tests.rs for the pretender
//! file-lines budget).

use super::*;

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
