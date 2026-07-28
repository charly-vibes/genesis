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
//! let fixture = Fixture::new()
//!     .with_marker(".wai")
//!     .with_marker(".dont")
//!     .with_file("hello.txt", "world")
//!     .build()
//!     .expect("build fixture");
//!
//! assert!(fixture.path(".wai").exists());
//! assert!(fixture.path("hello.txt").exists());
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::de::DeserializeOwned;

/// Errors that can occur while building a fixture or running a command in one.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// An empty command was passed to [`Fixture::run`].
    #[error("cannot run an empty command (no program given)")]
    EmptyCommand,
    /// A subprocess failed to spawn.
    #[error("failed to spawn `{program}`")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    /// `git` is not installed or a git operation failed.
    #[error("git operation `{stage}` failed")]
    Git {
        stage: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// An I/O error while laying out the fixture on disk.
    #[error("io error at {}: {message}", path.display())]
    Io { path: PathBuf, message: String },
    /// A config could not be serialized or written.
    #[error("config error")]
    Config(#[from] crate::config::ConfigError),
}

/// A scratch fixture: a temporary directory with test resources.
///
/// Created via the builder pattern. Automatically cleaned up on `Drop`.
#[derive(Debug)]
pub struct Fixture {
    /// The root of the temporary directory.
    root: PathBuf,
}

impl Fixture {
    /// Create a new [`FixtureBuilder`].
    ///
    /// Returns a builder (not a `Fixture`) so callers can chain `.with_*()`
    /// before `.build()`; this is the shape mandated by the fixture spec.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> FixtureBuilder {
        FixtureBuilder {
            markers: Vec::new(),
            files: Vec::new(),
            tomls: Vec::new(),
            configs: Vec::new(),
            git_init: false,
        }
    }

    /// The root path of the fixture.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path within the fixture.
    ///
    /// This is the spec-mandated path resolver: `fixture.path(".wai/config.toml")`
    /// returns `<fixture_root>/.wai/config.toml`. For the bare root, use
    /// [`Fixture::root`].
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Assert that a file exists at the given relative path.
    pub fn assert_file_exists(&self, relative: &str) {
        let path = self.path(relative);
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
            std::fs::read_to_string(self.path(relative)).unwrap_or_else(|_| String::new());
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
        let path = self.path(name);
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
        let path = self.path(relative);
        assert!(
            !path.exists(),
            "expected file to NOT exist: {}",
            path.display()
        );
    }

    /// Run a command as a subprocess in the fixture directory.
    ///
    /// Searches `PATH` for the binary. Returns stdout, stderr, and exit code.
    /// Fails gracefully (returns `Err`) if `args` is empty or the program
    /// cannot be spawned — it never panics.
    pub fn run(&self, args: &[&str]) -> Result<CommandOutput, FixtureError> {
        if args.is_empty() {
            return Err(FixtureError::EmptyCommand);
        }
        let program = args[0];
        let cmd_args = &args[1..];

        let output = Command::new(program)
            .args(cmd_args)
            .current_dir(&self.root)
            .output()
            .map_err(|e| FixtureError::Spawn {
                program: program.to_string(),
                source: e,
            })?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            signal: signal_of(&output.status),
        })
    }
}

/// The result of running a command in a fixture.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code (or -1 if the process was terminated by a signal).
    pub exit_code: i32,
    /// On Unix, the terminating signal if the process was killed by one;
    /// `None` otherwise (including on non-Unix platforms).
    pub signal: Option<i32>,
}

impl CommandOutput {
    /// Parse stdout as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&self.stdout).ok()
    }

    /// Returns `true` if the exit code is 0.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Extract the terminating signal from an [`ExitStatus`], if any.
#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new().build().expect("default fixture build")
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

/// A deferred config write captured at `with_config` time.
///
/// Stored as a closure so the typed config is written to `ConfigFile::path()`
/// at `build()` time, once the fixture root exists.
type ConfigWriter = Box<dyn FnOnce(&Path) -> Result<(), FixtureError>>;

struct ConfigEntry {
    write: ConfigWriter,
}

impl std::fmt::Debug for ConfigEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigEntry").finish_non_exhaustive()
    }
}

/// Builder for [`Fixture`].
#[derive(Debug)]
pub struct FixtureBuilder {
    markers: Vec<String>,
    files: Vec<FileEntry>,
    tomls: Vec<TomlEntry>,
    configs: Vec<ConfigEntry>,
    git_init: bool,
}

impl Default for FixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureBuilder {
    /// Create a new builder. Equivalent to [`Fixture::new`].
    pub fn new() -> Self {
        Self {
            markers: Vec::new(),
            files: Vec::new(),
            tomls: Vec::new(),
            configs: Vec::new(),
            git_init: false,
        }
    }

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

    /// Write a config file via the [`ConfigFile`](crate::config::ConfigFile) trait.
    ///
    /// The config is written at `build()` time to the path returned by
    /// `T::path(root)`, so a subsequent `ConfigFile::read(&fixture.root())`
    /// round-trips the same value.
    pub fn with_config<T>(mut self, config: T) -> Self
    where
        T: crate::config::ConfigFile + serde::Serialize + 'static,
    {
        self.configs.push(ConfigEntry {
            write: Box::new(move |root| {
                config.write(root)?;
                Ok(())
            }),
        });
        self
    }

    /// Initialize a git repository in the fixture.
    ///
    /// The actual `git init` (and initial commit) happens in [`build`], which
    /// returns [`Err(FixtureError::Git)`](FixtureError::Git) if `git` is not
    /// installed — callers can `.expect("git not available")` or skip the test.
    pub fn with_git_init(mut self) -> Self {
        self.git_init = true;
        self
    }

    /// Build the fixture, creating all directories and files.
    ///
    /// Returns `Err` on any I/O, config, or git failure rather than panicking.
    pub fn build(self) -> Result<Fixture, FixtureError> {
        // Create a temp dir that we own and clean up ourselves in `Drop`.
        // `keep()` consumes the TempDir, leaving the directory on disk and
        // transferring deletion responsibility to `Fixture::drop`.
        let root = tempfile::tempdir()
            .map_err(|e| FixtureError::Io {
                path: PathBuf::from("<tempdir>"),
                message: e.to_string(),
            })?
            .keep();

        // Create markers
        for marker in &self.markers {
            let path = root.join(marker);
            std::fs::create_dir_all(&path).map_err(|e| FixtureError::Io {
                path: path.clone(),
                message: format!("create marker '{}': {}", marker, e),
            })?;
        }

        // Write files
        for entry in &self.files {
            let path = root.join(&entry.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| FixtureError::Io {
                    path: parent.to_path_buf(),
                    message: format!("create parent for '{}': {}", entry.path, e),
                })?;
            }
            std::fs::write(&path, &entry.content).map_err(|e| FixtureError::Io {
                path: path.clone(),
                message: format!("write file '{}': {}", entry.path, e),
            })?;
        }

        // Write TOML files
        for entry in &self.tomls {
            let path = root.join(&entry.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| FixtureError::Io {
                    path: parent.to_path_buf(),
                    message: format!("create parent for '{}': {}", entry.path, e),
                })?;
            }
            let content = toml::to_string_pretty(&entry.value).map_err(|e| FixtureError::Io {
                path: path.clone(),
                message: format!("serialize TOML for '{}': {}", entry.path, e),
            })?;
            std::fs::write(&path, &content).map_err(|e| FixtureError::Io {
                path: path.clone(),
                message: format!("write TOML file '{}': {}", entry.path, e),
            })?;
        }

        // Write config files (deferred from with_config)
        for entry in self.configs {
            (entry.write)(&root)?;
        }

        // Git init
        if self.git_init {
            git_init_repo(&root)?;
        }

        Ok(Fixture { root })
    }
}

/// Run `git init` plus an initial commit in `root`.
fn git_init_repo(root: &Path) -> Result<(), FixtureError> {
    let run = |stage: &'static str, args: &[&str]| -> Result<std::process::Output, FixtureError> {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| FixtureError::Git { stage, source: e })
    };

    let init = run("init", &["init"])?;
    if !init.status.success() {
        return Err(FixtureError::Git {
            stage: "init",
            source: std::io::Error::other(format!(
                "git init exited {}: {}",
                init.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&init.stderr)
            )),
        });
    }

    // Seed a .gitkeep so the initial commit always has content.
    std::fs::write(root.join(".gitkeep"), "").map_err(|e| FixtureError::Git {
        stage: "seed",
        source: e,
    })?;

    // Configure a deterministic identity (ignore failures: not fatal).
    let _ = run("config", &["config", "user.email", "test@test.com"]);
    let _ = run("config", &["config", "user.name", "Test"]);

    let add = run("add", &["add", "-A"])?;
    if !add.status.success() {
        return Err(FixtureError::Git {
            stage: "add",
            source: std::io::Error::other(format!(
                "git add exited {}: {}",
                add.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&add.stderr)
            )),
        });
    }

    let commit = Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(root)
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
        .output()
        .map_err(|e| FixtureError::Git {
            stage: "commit",
            source: e,
        })?;
    if !commit.status.success() {
        return Err(FixtureError::Git {
            stage: "commit",
            source: std::io::Error::other(format!(
                "git commit exited {}: {}",
                commit.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&commit.stderr)
            )),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigFile;
    use serde::{Deserialize, Serialize};

    // -- Builder ------------------------------------------------------------

    #[test]
    fn test_with_marker_creates_directory() {
        let fixture = Fixture::new().with_marker(".wai").build().expect("build");
        fixture.assert_marker(".wai");
    }

    #[test]
    fn test_with_marker_dir_creates_nested_directory() {
        let fixture = Fixture::new()
            .with_marker_dir(".beads/hooks")
            .build()
            .expect("build");
        fixture.assert_marker(".beads/hooks");
    }

    #[test]
    fn test_with_file_creates_file() {
        let fixture = Fixture::new()
            .with_file("hello.txt", "world")
            .build()
            .expect("build");
        fixture.assert_file_exists("hello.txt");
        fixture.assert_file_contains("hello.txt", "world");
    }

    #[test]
    fn test_with_file_creates_nested_path() {
        let fixture = Fixture::new()
            .with_file("src/main.rs", "fn main() {}")
            .build()
            .expect("build");
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
            .expect("build");

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
            .expect("build");

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
            .expect("build");

        // Written to the path ConfigFile::path() declares...
        fixture.assert_file_exists(".fixture/config.toml");
        fixture.assert_file_contains(".fixture/config.toml", "alpha");

        // ...so ConfigFile::read() finds it and round-trips the value.
        let read = FixtureConfig::read(fixture.root()).expect("config read");
        assert_eq!(cfg, read);
    }

    #[test]
    fn test_with_config_uses_config_file_path_not_type_name() {
        // Regression guard for CORR-001: the config must NOT land in a
        // `::`-named directory derived from std::any::type_name.
        let fixture = Fixture::new()
            .with_config(FixtureConfig::default())
            .build()
            .expect("build");
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
        let fixture = Fixture::new().build().expect("build");
        fixture.assert_no_file("nonexistent.txt");
    }

    #[test]
    #[should_panic(expected = "expected file to exist")]
    fn test_assert_file_exists_panics_when_missing() {
        let fixture = Fixture::new().build().expect("build");
        fixture.assert_file_exists("missing.txt");
    }

    #[test]
    #[should_panic(expected = "expected file to NOT exist")]
    fn test_assert_no_file_panics_when_exists() {
        let fixture = Fixture::new()
            .with_file("present.txt", "")
            .build()
            .expect("build");
        fixture.assert_no_file("present.txt");
    }

    #[test]
    #[should_panic(expected = "expected file 'data.txt' to contain")]
    fn test_assert_file_contains_panics_on_mismatch() {
        let fixture = Fixture::new()
            .with_file("data.txt", "hello")
            .build()
            .expect("build");
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
        let status = fixture.run(&["git", "status"]).expect("git status");
        assert!(status.success(), "git status failed: {}", status.stderr);
    }

    // -- Path resolution ---------------------------------------------------

    #[test]
    fn test_path_resolution() {
        let fixture = Fixture::new()
            .with_file("sub/dir/file.txt", "content")
            .build()
            .expect("build");
        let resolved = fixture.path("sub/dir/file.txt");
        assert!(resolved.exists());
        assert!(resolved.is_file());
    }

    #[test]
    fn test_root_returns_fixture_root() {
        let fixture = Fixture::new().build().expect("build");
        assert!(fixture.root().exists());
        assert!(fixture.root().is_dir());
    }

    // -- Drop cleanup ------------------------------------------------------

    #[test]
    fn test_drop_cleans_up_temp_dir() {
        let path;
        {
            let fixture = Fixture::new().with_marker("temp").build().expect("build");
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
        let fixture = Fixture::new().build().expect("build");
        let output = fixture.run(&["echo", "hello world"]).expect("run");
        assert!(output.success());
        assert!(output.stdout.contains("hello world"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_failure_exit_code() {
        let fixture = Fixture::new().build().expect("build");
        let output = fixture.run(&["sh", "-c", "exit 42"]).expect("run");
        assert!(!output.success());
        assert_eq!(output.exit_code, 42);
        assert!(output.signal.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_run_with_stderr() {
        let fixture = Fixture::new().build().expect("build");
        let output = fixture
            .run(&["sh", "-c", "echo 'error msg' >&2"])
            .expect("run");
        assert!(output.success());
        assert!(output.stderr.contains("error msg"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_json_output() {
        let fixture = Fixture::new().build().expect("build");
        let output = fixture.run(&["echo", "{\"key\":\"value\"}"]).expect("run");
        let parsed: Option<serde_json::Value> = output.json();
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap()["key"], "value");
    }

    #[cfg(unix)]
    #[test]
    fn test_run_in_fixture_directory() {
        let fixture = Fixture::new()
            .with_file("test.txt", "fixture content")
            .build()
            .expect("build");
        let output = fixture.run(&["cat", "test.txt"]).expect("run");
        assert!(output.success());
        assert!(output.stdout.contains("fixture content"));
    }

    #[test]
    fn test_run_empty_args_returns_err() {
        let fixture = Fixture::new().build().expect("build");
        let result = fixture.run(&[]);
        assert!(matches!(result, Err(FixtureError::EmptyCommand)));
    }

    #[test]
    fn test_run_non_existent_command_returns_err() {
        let fixture = Fixture::new().build().expect("build");
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
}
