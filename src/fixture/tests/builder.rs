//! Tests for the [`FixtureBuilder`] API: markers, files, TOML, configs,
//! path resolution, and drop cleanup.

// pretender: role=test
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::ConfigFile;
use crate::fixture::Fixture;
use crate::testutil::TestUnwrap;

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
    assert_eq!(
        std::fs::read_to_string(fixture.root().join("hello.txt")).require("read hello.txt"),
        "world",
        "with_file must write the exact content"
    );
}

#[test]
fn test_with_file_creates_nested_path() {
    let fixture = Fixture::new()
        .with_file("src/main.rs", "fn main() {}")
        .build()
        .require("build");
    fixture.assert_file_exists("src/main.rs");
    fixture.assert_file_contains("src/main.rs", "fn main()");
    assert!(
        fixture.root().join("src/main.rs").is_file(),
        "with_file must create parent directories"
    );
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
    let raw = std::fs::read_to_string(fixture.root().join("config.toml")).require("read toml");
    assert!(
        raw.contains("name") && raw.contains("true"),
        "generated TOML must declare the table contents: {raw:?}"
    );
}

#[test]
fn test_with_toml_round_trips_values() {
    let mut map = toml::map::Map::new();
    map.insert("name".into(), toml::Value::String("test".into()));
    map.insert("enabled".into(), toml::Value::Boolean(true));

    let fixture = Fixture::new()
        .with_toml("config.toml", toml::Value::Table(map))
        .build()
        .require("build");

    let raw = std::fs::read_to_string(fixture.root().join("config.toml")).require("read toml");
    let parsed: toml::Value = toml::from_str(&raw).require("parse generated toml");
    assert_eq!(
        parsed["name"].as_str(),
        Some("test"),
        "with_toml must round-trip values"
    );
    assert_eq!(
        parsed["enabled"].as_bool(),
        Some(true),
        "with_toml must round-trip bools"
    );
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
    assert!(
        fixture.root().join(".wai").is_dir() && fixture.root().join(".dont").is_dir(),
        "both markers must exist"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root().join("a.txt")).require("read a.txt"),
        "aaa",
        "multiple with_file entries must all be written"
    );
}

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

/// The config must land exactly where `ConfigFile::path()` declares, so that
/// `ConfigFile::read()` finds it and round-trips the value.
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

    fixture.assert_file_exists(".fixture/config.toml");
    fixture.assert_file_contains(".fixture/config.toml", "alpha");

    let read = FixtureConfig::read(fixture.root()).require("config read");
    assert_eq!(cfg, read);
}

/// Regression guard for CORR-001: the config must NOT land in a
/// `::`-named directory derived from std::any::type_name.
#[test]
fn test_with_config_uses_config_file_path_not_type_name() {
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

#[test]
fn test_drop_cleans_up_temp_dir() {
    let path;
    {
        let fixture = Fixture::new().with_marker("temp").build().require("build");
        path = fixture.root().to_path_buf();
        assert!(path.exists());
    }
    assert!(!path.exists(), "temp dir should be cleaned up on drop");
}
