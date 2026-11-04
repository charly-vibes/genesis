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

        // Hermetic: hook environments inject `GIT_DIR` and friends (git
        // exports them to hooks), which would hijack fixture-run git onto
        // the real repository. Strip them — fixtures are self-contained.
        let output = hermetic_git(&mut Command::new(program))
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

/// Remove the hook-context variables git exports to hook processes
/// (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`) from a fixture command.
///
/// Rationale: when the test suite runs inside a git hook (the pre-commit
/// testaruda selection runs the whole suite), the surrounding `git
/// commit` exports `GIT_DIR` — and a fixture that inherits it would run
/// its `git init`/`add`/`commit` against the *real* repository instead
/// of the fixture's own (observed: a hook-time cargo test landed a bogus
/// `initial` commit on a development branch). Fixtures are self-contained
/// sandboxes; their git operations must discover the fixture repo from
/// the command's working directory alone.
fn hermetic_git(cmd: &mut Command) -> &mut Command {
    for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
        cmd.env_remove(var);
    }
    cmd
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
        hermetic_git(&mut Command::new("git"))
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

    let commit = hermetic_git(&mut Command::new("git"))
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
#[cfg(test)]
#[path = "fixture/tests.rs"]
mod tests;
