//! Unit tests for `fixture` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use crate::config::ConfigFile;
use serde::{Deserialize, Serialize};

// -- Builder ------------------------------------------------------------

#[test]
fn test_with_marker_creates_directory() {
    let fixture = Fixture::new().with_marker(".wai").build().require("build");
    fixture.assert_marker(".wai");
    assert!(
        fixture.root().join(".wai").is_dir(),
        "marker must be created as a directory"
    );
}

#[test]
fn test_with_marker_dir_creates_nested_directory() {
    let fixture = Fixture::new()
        .with_marker_dir(".beads/hooks")
        .build()
        .require("build");
    fixture.assert_marker(".beads/hooks");
    assert!(
        fixture.root().join(".beads/hooks").is_dir(),
        "nested marker must be created as a directory"
    );
}

#[test]
fn test_with_file_creates_file() {
    let fixture = Fixture::new()
        .with_file("hello.txt", "world")
        .build()
        .require("build");
    fixture.assert_file_exists("hello.txt");
    fixture.assert_file_contains("hello.txt", "world");
}

#[test]
fn test_with_file_creates_nested_path() {
    let fixture = Fixture::new()
        .with_file("src/main.rs", "fn main() {}")
        .build()
        .require("build");
    fixture.assert_file_exists("src/main.rs");
    fixture.assert_file_contains("src/main.rs", "fn main()");
}

#[test]
fn test_with_toml_creates_valid_toml() {
    let mut map = toml::map::Map::new();
    map.insert("name".into(), toml::Value::String("test".into()));
    map.insert("enabled".into(), toml::Value::Boolean(true));

    let fixture = Fixture::new()
        .with_toml("config.toml", toml::Value::Table(map))
        .build()
        .require("build");

    fixture.assert_file_exists("config.toml");
    fixture.assert_file_contains("config.toml", "name");
    fixture.assert_file_contains("config.toml", "true");
}

#[test]
fn test_multiple_markers_and_files() {
    let fixture = Fixture::new()
        .with_marker(".wai")
        .with_marker(".dont")
        .with_file("a.txt", "aaa")
        .with_file("b.txt", "bbb")
        .build()
        .require("build");

    fixture.assert_marker(".wai");
    fixture.assert_marker(".dont");
    fixture.assert_file_exists("a.txt");
    fixture.assert_file_exists("b.txt");
}

// -- with_config (round-trip via ConfigFile) ---------------------------

/// A typed config used to verify `with_config` round-trips through
/// `ConfigFile::path` / `ConfigFile::read`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
struct FixtureConfig {
    name: String,
    threshold: f64,
    enabled: bool,
}

impl ConfigFile for FixtureConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".fixture/config.toml")
    }
}

#[test]
fn test_with_config_round_trips_through_config_file() {
    let cfg = FixtureConfig {
        name: "alpha".to_string(),
        threshold: 0.5,
        enabled: true,
    };
    let fixture = Fixture::new()
        .with_config(cfg.clone())
        .build()
        .require("build");

    // Written to the path ConfigFile::path() declares...
    fixture.assert_file_exists(".fixture/config.toml");
    fixture.assert_file_contains(".fixture/config.toml", "alpha");

    // ...so ConfigFile::read() finds it and round-trips the value.
    let read = FixtureConfig::read(fixture.root()).require("config read");
    assert_eq!(cfg, read);
}

#[test]
fn test_with_config_uses_config_file_path_not_type_name() {
    // Regression guard for CORR-001: the config must NOT land in a
    // `::`-named directory derived from std::any::type_name.
    let fixture = Fixture::new()
        .with_config(FixtureConfig::default())
        .build()
        .require("build");
    let bad = std::any::type_name::<FixtureConfig>();
    assert!(
        !fixture.path(bad).exists(),
        "config was written to the type_name path {}; should be .fixture/config.toml",
        bad
    );
}

// -- Assertions --------------------------------------------------------

#[test]
fn test_assert_no_file_passes_when_missing() {
    let fixture = Fixture::new().build().require("build");
    fixture.assert_no_file("nonexistent.txt");
}

#[test]
#[should_panic(expected = "expected file to exist")]
fn test_assert_file_exists_panics_when_missing() {
    let fixture = Fixture::new().build().require("build");
    fixture.assert_file_exists("missing.txt");
}

#[test]
#[should_panic(expected = "expected file to NOT exist")]
fn test_assert_no_file_panics_when_exists() {
    let fixture = Fixture::new()
        .with_file("present.txt", "")
        .build()
        .require("build");
    fixture.assert_no_file("present.txt");
}

#[test]
#[should_panic(expected = "expected file 'data.txt' to contain")]
fn test_assert_file_contains_panics_on_mismatch() {
    let fixture = Fixture::new()
        .with_file("data.txt", "hello")
        .build()
        .require("build");
    fixture.assert_file_contains("data.txt", "world");
}

// -- Git init ----------------------------------------------------------

#[test]
fn test_with_git_init_creates_repo() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(FixtureError::Git { .. }) => return, // git not available
        Err(e) => panic!("unexpected build error: {e:?}"),
    };
    fixture.assert_marker(".git");
    // `git status` must succeed inside the fixture.
    let status = fixture.run(&["git", "status"]).require("git status");
    assert!(status.success(), "git status failed: {}", status.stderr);
}

// -- Path resolution ---------------------------------------------------

#[test]
fn test_path_resolution() {
    let fixture = Fixture::new()
        .with_file("sub/dir/file.txt", "content")
        .build()
        .require("build");
    let resolved = fixture.path("sub/dir/file.txt");
    assert!(resolved.exists());
    assert!(resolved.is_file());
}

#[test]
fn test_root_returns_fixture_root() {
    let fixture = Fixture::new().build().require("build");
    assert!(fixture.root().exists());
    assert!(fixture.root().is_dir());
}

// -- Drop cleanup ------------------------------------------------------

#[test]
fn test_drop_cleans_up_temp_dir() {
    let path;
    {
        let fixture = Fixture::new().with_marker("temp").build().require("build");
        path = fixture.root().to_path_buf();
        assert!(path.exists());
    }
    // After Fixture is dropped, the temp dir should be gone
    assert!(!path.exists(), "temp dir should be cleaned up on drop");
}

// -- Fixture::run() ---------------------------------------------------

#[cfg(unix)]
#[test]
fn test_run_echo_command() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["echo", "hello world"]).require("run");
    assert!(output.success());
    assert!(output.stdout.contains("hello world"));
}

#[cfg(unix)]
#[test]
fn test_run_failure_exit_code() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["sh", "-c", "exit 42"]).require("run");
    assert!(!output.success());
    assert_eq!(output.exit_code, 42);
    assert!(output.signal.is_none());
}

#[cfg(unix)]
#[test]
fn test_run_with_stderr() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture
        .run(&["sh", "-c", "echo 'error msg' >&2"])
        .require("run");
    assert!(output.success());
    assert!(output.stderr.contains("error msg"));
}

#[cfg(unix)]
#[test]
fn test_run_json_output() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["echo", "{\"key\":\"value\"}"]).require("run");
    let parsed: Option<serde_json::Value> = output.json();
    assert!(parsed.is_some());
    assert_eq!(parsed.require("value")["key"], "value");
}

#[cfg(unix)]
#[test]
fn test_run_in_fixture_directory() {
    let fixture = Fixture::new()
        .with_file("test.txt", "fixture content")
        .build()
        .require("build");
    let output = fixture.run(&["cat", "test.txt"]).require("run");
    assert!(output.success());
    assert!(output.stdout.contains("fixture content"));
}

#[test]
fn test_run_empty_args_returns_err() {
    let fixture = Fixture::new().build().require("build");
    let result = fixture.run(&[]);
    assert!(matches!(result, Err(FixtureError::EmptyCommand)));
}

#[test]
fn test_run_non_existent_command_returns_err() {
    let fixture = Fixture::new().build().require("build");
    let result = fixture.run(&["nonexistent_command_xyz123"]);
    assert!(matches!(result, Err(FixtureError::Spawn { .. })));
}

// -- CommandOutput -----------------------------------------------------

#[test]
fn test_command_output_success() {
    let ok = CommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
        signal: None,
    };
    assert!(ok.success());

    let fail = CommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 1,
        signal: None,
    };
    assert!(!fail.success());
}

#[test]
fn test_command_output_json_invalid() {
    let output = CommandOutput {
        stdout: "not json".into(),
        stderr: String::new(),
        exit_code: 0,
        signal: None,
    };
    let result: Option<serde_json::Value> = output.json();
    assert!(result.is_none());
}

// -- FixtureError plumbing --------------------------------------------

#[test]
fn test_fixture_error_display_non_empty() {
    let e = FixtureError::EmptyCommand;
    assert!(!e.to_string().is_empty());
}
