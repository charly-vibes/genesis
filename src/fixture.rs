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

use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

mod command;
mod entries;
mod git_init;

pub use command::CommandOutput;
use command::hermetic_git;
#[cfg(unix)]
use command::signal_of;
use entries::{
    ConfigEntry, FileEntry, TomlEntry, create_markers, write_plain_files, write_toml_files,
};
use git_init::git_init_repo;

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

        create_markers(&root, &self.markers)?;
        write_plain_files(&root, &self.files)?;
        write_toml_files(&root, &self.tomls)?;

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

#[cfg(test)]
#[cfg(test)]
#[path = "fixture/tests.rs"]
mod tests;
