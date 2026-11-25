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

mod command;
mod entries;
mod error;
mod git_init;

pub use command::CommandOutput;
pub use error::FixtureError;

use std::path::{Path, PathBuf};

use entries::{
    ConfigEntry, FileEntry, TomlEntry, create_markers, write_configs, write_plain_files,
    write_toml_files,
};
use git_init::git_init_repo;

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
        let Some((program, cmd_args)) = args.split_first() else {
            return Err(FixtureError::EmptyCommand);
        };
        let output = command::spawn_in(program, cmd_args, &self.root)?;
        Ok(CommandOutput::from_process(&output))
    }
}

/// Create the fixture root as an owned temp directory.
///
/// `TempDir::keep()` consumes the `TempDir`, leaving the directory on disk
/// and transferring deletion responsibility to [`Fixture::drop`].
fn create_root() -> Result<PathBuf, FixtureError> {
    tempfile::tempdir()
        .map_err(|e| FixtureError::Io {
            path: PathBuf::from("<tempdir>"),
            message: e.to_string(),
        })
        .map(tempfile::TempDir::keep)
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
        let root = create_root()?;
        create_markers(&root, &self.markers)?;
        write_plain_files(&root, &self.files)?;
        write_toml_files(&root, &self.tomls)?;
        write_configs(&root, self.configs)?;
        if self.git_init {
            git_init_repo(&root)?;
        }
        Ok(Fixture { root })
    }
}

#[cfg(test)]
mod tests;
