//! Init scaffolding helpers — shared directory, config, and file creation.
//!
//! Tools can use [`Scaffold`] to standardize their `init` command:
//! create directories, write default configs, inject managed blocks,
//! and set up `.gitignore` entries — all from a single builder.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use genesis::scaffold::Scaffold;
//! use std::path::Path;
//!
//! let result = Scaffold::new("/tmp/my-project")
//!     .dir(".tool/state")
//!     .dir(".tool/cache")
//!     .default_config("tool.toml", "key = \"value\"\n")
//!     .gitignore_entry(".tool/cache/")
//!     .agent_command_file(".claude/commands/my-tool.md", "# My Tool\n\nUsage...")
//!     .build()
//!     .unwrap();
//!
//! for path in &result.created {
//!     println!("created: {:?}", path);
//! }
//! ```

use std::path::{Path, PathBuf};

/// Result of a [`Scaffold`] build.
#[derive(Debug, Clone)]
pub struct ScaffoldResult {
    /// All paths that were created (directories and files).
    pub created: Vec<PathBuf>,
    /// Paths that already existed (no-op).
    pub existed: Vec<PathBuf>,
}

/// A builder for standard init scaffolding.
///
/// Collects operations and then applies them all in [`build`](Scaffold::build).
#[derive(Debug)]
pub struct Scaffold {
    project_root: PathBuf,
    dirs: Vec<PathBuf>,
    files: Vec<(PathBuf, String)>,
    gitignore_entries: Vec<String>,
}

impl Scaffold {
    /// Start building a scaffold at the given project root.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            dirs: Vec::new(),
            files: Vec::new(),
            gitignore_entries: Vec::new(),
        }
    }

    /// Ensure a directory exists relative to the project root.
    ///
    /// Directories are created with `mkdir -p` semantics.
    pub fn dir(mut self, path: impl AsRef<Path>) -> Self {
        self.dirs.push(PathBuf::from(path.as_ref()));
        self
    }

    /// Write a default config file relative to the project root.
    ///
    /// The file is only created if it doesn't already exist (won't overwrite).
    pub fn default_config(mut self, path: impl AsRef<Path>, content: impl Into<String>) -> Self {
        self.files
            .push((PathBuf::from(path.as_ref()), content.into()));
        self
    }

    /// Add a `.gitignore` entry (appended to `.gitignore` if it exists, or
    /// created as a new file).
    pub fn gitignore_entry(mut self, entry: impl Into<String>) -> Self {
        self.gitignore_entries.push(entry.into());
        self
    }

    /// Convenience: create a managed block file.
    ///
    /// Shortcut for adding a `default_config` with managed block markers.
    pub fn managed_block(
        self,
        block_name: impl AsRef<str>,
        path: impl AsRef<Path>,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        let wrapped = format!(
            "<!-- {}:START -->\n{}\n<!-- {}:END -->\n",
            block_name.as_ref(),
            content.trim(),
            block_name.as_ref(),
        );
        self.default_config(path, wrapped)
    }

    /// Convenience: create an agent command file (e.g., `.claude/commands/`).
    ///
    /// Creates the parent directory and writes the file if it doesn't exist.
    pub fn agent_command_file(self, path: impl AsRef<Path>, content: impl Into<String>) -> Self {
        let path = PathBuf::from(path.as_ref());
        if let Some(parent) = path.parent() {
            self.dir(parent).default_config(path, content)
        } else {
            self.default_config(path, content)
        }
    }

    /// Apply all collected operations.
    ///
    /// Creates directories first, then writes files, then updates `.gitignore`.
    /// Files are only created if they don't already exist (no overwrites).
    pub fn build(self) -> std::io::Result<ScaffoldResult> {
        let mut created = Vec::new();
        let mut existed = Vec::new();

        // 1. Create directories
        for dir in &self.dirs {
            let abs_path = self.project_root.join(dir);
            if abs_path.exists() {
                existed.push(abs_path);
            } else {
                std::fs::create_dir_all(&abs_path)?;
                created.push(abs_path);
            }
        }

        // 2. Write files (no overwrite)
        for (rel_path, content) in &self.files {
            let abs_path = self.project_root.join(rel_path);
            if abs_path.exists() {
                existed.push(abs_path);
            } else {
                if let Some(parent) = abs_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&abs_path, content)?;
                created.push(abs_path);
            }
        }

        // 3. Update .gitignore
        if !self.gitignore_entries.is_empty() {
            let gitignore_path = self.project_root.join(".gitignore");
            let gitignore_existed = gitignore_path.exists();

            // Ensure parent directory exists
            if let Some(parent) = gitignore_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let existing = if gitignore_existed {
                let mut prior = std::fs::read_to_string(&gitignore_path)?;
                if !prior.ends_with('\n') {
                    prior.push('\n');
                }
                prior
            } else {
                String::new()
            };

            let new_entries: Vec<String> = self
                .gitignore_entries
                .iter()
                .map(|entry| format!("{}\n", entry))
                .filter(|line| !existing.contains(line.as_str()) && !existing.contains(line.trim()))
                .collect();

            if !new_entries.is_empty() {
                let content = format!("{existing}{}", new_entries.concat());
                std::fs::write(&gitignore_path, content)?;
                if gitignore_existed {
                    existed.push(gitignore_path);
                } else {
                    created.push(gitignore_path);
                }
            } else if gitignore_existed {
                existed.push(gitignore_path);
            } else {
                // gitignore was created but had no new entries
                // Keep it though — the file is now present
                created.push(gitignore_path);
            }
        }

        Ok(ScaffoldResult { created, existed })
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "scaffold/tests.rs"]
mod tests;
