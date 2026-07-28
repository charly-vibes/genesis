//! Scratch fixtures for testing and dogfooding.
//!
//! Provides `Fixture` — a builder for temporary directories with
//! markers, configs, files, and git initialization.
//!
//! ## Usage
//!
//! ```rust
//! use genesis::fixture::Fixture;
//!
//! let fixture = Fixture::builder()
//!     .with_marker(".wai")
//!     .with_marker(".dont")
//!     .with_file("hello.txt", "world")
//!     .build();
//!
//! assert!(fixture.path().join(".wai").exists());
//! assert!(fixture.path().join("hello.txt").exists());
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

/// A scratch fixture: a temporary directory with test resources.
///
/// Created via the builder pattern. Automatically cleaned up on `Drop`.
#[derive(Debug)]
pub struct Fixture {
    /// The root of the temporary directory.
    root: PathBuf,
    /// Whether the directory was git-initialized.
    #[allow(dead_code)]
    has_git: bool,
}

impl Fixture {
    /// Create a new `FixtureBuilder`.
    pub fn builder() -> FixtureBuilder {
        FixtureBuilder {
            markers: Vec::new(),
            files: Vec::new(),
            tomls: Vec::new(),
            configs: Vec::new(),
            git_init: false,
        }
    }

    /// The root path of the fixture.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path within the fixture.
    pub fn join(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Assert that a file exists at the given relative path.
    pub fn assert_file_exists(&self, relative: &str) {
        let path = self.join(relative);
        assert!(path.exists(), "expected file to exist: {}", path.display());
        assert!(
            path.is_file(),
            "expected path to be a file: {}",
            path.display()
        );
    }

    /// Assert that a file contains the given substring.
    pub fn assert_file_contains(&self, relative: &str, pattern: &str) {
        self.assert_file_exists(relative);
        let content =
            std::fs::read_to_string(self.join(relative)).unwrap_or_else(|_| String::new());
        assert!(
            content.contains(pattern),
            "expected file '{}' to contain '{}', got: {:?}",
            relative,
            pattern,
            content
        );
    }

    /// Assert that a marker directory exists.
    pub fn assert_marker(&self, name: &str) {
        let path = self.join(name);
        assert!(
            path.exists(),
            "expected marker to exist: {}",
            path.display()
        );
        assert!(
            path.is_dir(),
            "expected marker to be a directory: {}",
            path.display()
        );
    }

    /// Assert that a file does NOT exist.
    pub fn assert_no_file(&self, relative: &str) {
        let path = self.join(relative);
        assert!(
            !path.exists(),
            "expected file to NOT exist: {}",
            path.display()
        );
    }

    /// Run a command as a subprocess in the fixture directory.
    ///
    /// Searches `PATH` for the binary. Returns stdout, stderr, and exit code.
    pub fn run(&self, args: &[&str]) -> CommandOutput {
        let (program, cmd_args) = if args.is_empty() {
            ("", &[] as &[&str])
        } else {
            (args[0], &args[1..])
        };

        let output = Command::new(program)
            .args(cmd_args)
            .current_dir(&self.root)
            .output()
            .unwrap_or_else(|e| panic!("failed to run '{}' in fixture: {}", args.join(" "), e));

        CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        }
    }
}

/// The result of running a command in a fixture.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code (or -1 if terminated by signal).
    pub exit_code: i32,
}

impl CommandOutput {
    /// Parse stdout as JSON.
    pub fn json<'a, T: serde::Deserialize<'a>>(&'a self) -> Option<T> {
        serde_json::from_str(&self.stdout).ok()
    }

    /// Returns `true` if the exit code is 0.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A file entry in the builder.
#[derive(Debug)]
struct FileEntry {
    path: String,
    content: String,
}

/// A TOML file entry in the builder.
#[derive(Debug)]
struct TomlEntry {
    path: String,
    value: toml::Value,
}

/// A config file entry.
#[derive(Debug)]
struct ConfigEntry {
    tool_name: String,
    /// The serialized config content (already known at build time).
    content: String,
}

/// Builder for `Fixture`.
#[derive(Debug)]
pub struct FixtureBuilder {
    markers: Vec<String>,
    files: Vec<FileEntry>,
    tomls: Vec<TomlEntry>,
    configs: Vec<ConfigEntry>,
    git_init: bool,
}

impl FixtureBuilder {
    /// Add a marker directory (e.g., `.wai`, `.dont`).
    pub fn with_marker(mut self, name: &str) -> Self {
        self.markers.push(name.to_string());
        self
    }

    /// Add a nested marker directory (e.g., `.beads/hooks`).
    pub fn with_marker_dir(mut self, path: &str) -> Self {
        self.markers.push(path.to_string());
        self
    }

    /// Write a file at the given relative path.
    pub fn with_file(mut self, path: &str, content: &str) -> Self {
        self.files.push(FileEntry {
            path: path.to_string(),
            content: content.to_string(),
        });
        self
    }

    /// Write a serialized TOML file.
    pub fn with_toml(mut self, path: &str, value: toml::Value) -> Self {
        self.tomls.push(TomlEntry {
            path: path.to_string(),
            value,
        });
        self
    }

    /// Write a config file via the `ConfigFile` trait.
    ///
    /// The config is written to the path returned by `ConfigFile::path()`.
    pub fn with_config<T: crate::config::ConfigFile + serde::Serialize>(
        mut self,
        config: &T,
    ) -> Self {
        // We can't call ConfigFile::write() here because we don't have
        // the repo root yet (it's created in build()). Instead, we
        // serialize the config and store it for later.
        let content = toml::to_string(config).unwrap_or_else(|_| String::new());
        self.configs.push(ConfigEntry {
            tool_name: std::any::type_name::<T>().to_string(),
            content,
        });
        self
    }

    /// Initialize a git repository in the fixture.
    ///
    /// Returns an error if `git` is not installed.
    pub fn with_git_init(mut self) -> Self {
        self.git_init = true;
        self
    }

    /// Build the fixture, creating all directories and files.
    ///
    /// # Panics
    ///
    /// Panics if `with_git_init` was called but `git` is not installed.
    #[allow(deprecated)]
    pub fn build(self) -> Fixture {
        // Use tempfile::Builder to create a temp dir that we manage manually.
        // into_path() is deprecated — use Builder::keep(true) instead.
        let root = tempfile::Builder::new()
            .keep(true)
            .tempdir()
            .expect("failed to create temp dir")
            .into_path();

        // Create markers
        for marker in &self.markers {
            let path = root.join(marker);
            std::fs::create_dir_all(&path)
                .unwrap_or_else(|e| panic!("failed to create marker '{}': {}", marker, e));
        }

        // Write files
        for entry in &self.files {
            let path = root.join(&entry.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("failed to create parent dir for '{}': {}", entry.path, e)
                });
            }
            std::fs::write(&path, &entry.content)
                .unwrap_or_else(|e| panic!("failed to write file '{}': {}", entry.path, e));
        }

        // Write TOML files
        for entry in &self.tomls {
            let path = root.join(&entry.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("failed to create parent dir for '{}': {}", entry.path, e)
                });
            }
            let content = toml::to_string_pretty(&entry.value)
                .unwrap_or_else(|e| panic!("failed to serialize TOML for '{}': {}", entry.path, e));
            std::fs::write(&path, &content)
                .unwrap_or_else(|e| panic!("failed to write TOML file '{}': {}", entry.path, e));
        }

        // Write config files
        for entry in &self.configs {
            // Use a heuristic path: <tool_name>/config.toml
            let config_dir = root.join(&entry.tool_name);
            std::fs::create_dir_all(&config_dir).unwrap_or_else(|e| {
                panic!(
                    "failed to create config dir for '{}': {}",
                    entry.tool_name, e
                )
            });
            let config_path = config_dir.join("config.toml");
            std::fs::write(&config_path, &entry.content).unwrap_or_else(|e| {
                panic!("failed to write config for '{}': {}", entry.tool_name, e)
            });
        }

        // Git init
        if self.git_init {
            let status = Command::new("git")
                .args(["init"])
                .current_dir(&root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("git not found — install git to use with_git_init()");

            assert!(status.success(), "git init failed");

            // Initial commit
            let _ = Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(&root)
                .output();
            let _ = Command::new("git")
                .args(["config", "user.name", "Test"])
                .current_dir(&root)
                .output();
            let _ = Command::new("git")
                .args(["add", "-A"])
                .current_dir(&root)
                .output();
            let _ = Command::new("git")
                .args(["commit", "-m", "initial"])
                .current_dir(&root)
                .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00Z")
                .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output();
        }

        Fixture {
            root,
            has_git: self.git_init,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Builder ------------------------------------------------------------

    #[test]
    fn test_with_marker_creates_directory() {
        let fixture = Fixture::builder().with_marker(".wai").build();
        fixture.assert_marker(".wai");
    }

    #[test]
    fn test_with_marker_dir_creates_nested_directory() {
        let fixture = Fixture::builder().with_marker_dir(".beads/hooks").build();
        fixture.assert_marker(".beads/hooks");
    }

    #[test]
    fn test_with_file_creates_file() {
        let fixture = Fixture::builder().with_file("hello.txt", "world").build();
        fixture.assert_file_exists("hello.txt");
        fixture.assert_file_contains("hello.txt", "world");
    }

    #[test]
    fn test_with_file_creates_nested_path() {
        let fixture = Fixture::builder()
            .with_file("src/main.rs", "fn main() {}")
            .build();
        fixture.assert_file_exists("src/main.rs");
        fixture.assert_file_contains("src/main.rs", "fn main()");
    }

    #[test]
    fn test_with_toml_creates_valid_toml() {
        let mut map = toml::map::Map::new();
        map.insert("name".into(), toml::Value::String("test".into()));
        map.insert("enabled".into(), toml::Value::Boolean(true));

        let fixture = Fixture::builder()
            .with_toml("config.toml", toml::Value::Table(map))
            .build();

        fixture.assert_file_exists("config.toml");
        fixture.assert_file_contains("config.toml", "name");
        fixture.assert_file_contains("config.toml", "true");
    }

    #[test]
    fn test_multiple_markers_and_files() {
        let fixture = Fixture::builder()
            .with_marker(".wai")
            .with_marker(".dont")
            .with_file("a.txt", "aaa")
            .with_file("b.txt", "bbb")
            .build();

        fixture.assert_marker(".wai");
        fixture.assert_marker(".dont");
        fixture.assert_file_exists("a.txt");
        fixture.assert_file_exists("b.txt");
    }

    // -- Assertions --------------------------------------------------------

    #[test]
    fn test_assert_no_file_passes_when_missing() {
        let fixture = Fixture::builder().build();
        fixture.assert_no_file("nonexistent.txt");
    }

    #[test]
    #[should_panic(expected = "expected file to exist")]
    fn test_assert_file_exists_panics_when_missing() {
        let fixture = Fixture::builder().build();
        fixture.assert_file_exists("missing.txt");
    }

    #[test]
    #[should_panic(expected = "expected file to NOT exist")]
    fn test_assert_no_file_panics_when_exists() {
        let fixture = Fixture::builder().with_file("present.txt", "").build();
        fixture.assert_no_file("present.txt");
    }

    #[test]
    #[should_panic(expected = "expected file 'data.txt' to contain")]
    fn test_assert_file_contains_panics_on_mismatch() {
        let fixture = Fixture::builder().with_file("data.txt", "hello").build();
        fixture.assert_file_contains("data.txt", "world");
    }

    // -- Git init ----------------------------------------------------------

    #[test]
    fn test_with_git_init_creates_repo() {
        let fixture = Fixture::builder().with_git_init().build();
        fixture.assert_marker(".git");
    }

    // -- Path resolution ---------------------------------------------------

    #[test]
    fn test_path_resolution() {
        let fixture = Fixture::builder()
            .with_file("sub/dir/file.txt", "content")
            .build();
        let resolved = fixture.join("sub/dir/file.txt");
        assert!(resolved.exists());
        assert!(resolved.is_file());
    }

    // -- Drop cleanup ------------------------------------------------------

    #[test]
    fn test_drop_cleans_up_temp_dir() {
        let path;
        {
            let fixture = Fixture::builder().with_marker("temp").build();
            path = fixture.path().to_path_buf();
            assert!(path.exists());
        }
        // After Fixture is dropped, the temp dir should be gone
        assert!(!path.exists(), "temp dir should be cleaned up on drop");
    }

    // -- Fixture::run() ---------------------------------------------------

    #[test]
    fn test_run_echo_command() {
        let fixture = Fixture::builder().build();
        let output = fixture.run(&["echo", "hello world"]);
        assert!(output.success());
        assert!(output.stdout.contains("hello world"));
    }

    #[test]
    fn test_run_failure_exit_code() {
        let fixture = Fixture::builder().build();
        let output = fixture.run(&["sh", "-c", "exit 42"]);
        assert!(!output.success());
        assert_eq!(output.exit_code, 42);
    }

    #[test]
    fn test_run_with_stderr() {
        let fixture = Fixture::builder().build();
        let output = fixture.run(&["sh", "-c", "echo 'error msg' >&2"]);
        assert!(output.success());
        assert!(output.stderr.contains("error msg"));
    }

    #[test]
    fn test_run_json_output() {
        let fixture = Fixture::builder().build();
        let output = fixture.run(&["echo", "{\"key\":\"value\"}"]);
        let parsed: Option<serde_json::Value> = output.json();
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap()["key"], "value");
    }

    #[test]
    fn test_run_in_fixture_directory() {
        let fixture = Fixture::builder()
            .with_file("test.txt", "fixture content")
            .build();
        let output = fixture.run(&["cat", "test.txt"]);
        assert!(output.success());
        assert!(output.stdout.contains("fixture content"));
    }

    #[test]
    fn test_run_non_existent_command() {
        let fixture = Fixture::builder().build();
        let result = std::panic::catch_unwind(|| {
            fixture.run(&["nonexistent_command_xyz123"]);
        });
        assert!(result.is_err(), "should panic on missing command");
    }

    // -- CommandOutput -----------------------------------------------------

    #[test]
    fn test_command_output_success() {
        let ok = CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert!(ok.success());

        let fail = CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 1,
        };
        assert!(!fail.success());
    }

    #[test]
    fn test_command_output_json_invalid() {
        let output = CommandOutput {
            stdout: "not json".into(),
            stderr: String::new(),
            exit_code: 0,
        };
        let result: Option<serde_json::Value> = output.json();
        assert!(result.is_none());
    }
}
