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

            let mut existing = String::new();
            if gitignore_existed {
                existing = std::fs::read_to_string(&gitignore_path)?;
                if !existing.ends_with('\n') {
                    existing.push('\n');
                }
            }

            let mut new_entries = Vec::new();
            for entry in &self.gitignore_entries {
                let line = format!("{}\n", entry);
                if !existing.contains(&line) && !existing.contains(entry.as_str()) {
                    new_entries.push(line);
                }
            }

            if !new_entries.is_empty() {
                let mut content = existing;
                for entry in &new_entries {
                    content.push_str(entry);
                }
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
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn project_root(dir: &TempDir) -> PathBuf {
        dir.path().join("my-project")
    }

    // ── Basic scaffolding ─────────────────────────────────────────────

    #[test]
    fn test_create_directories() {
        let dir = tmp();
        let root = project_root(&dir);

        let result = Scaffold::new(&root)
            .dir(".tool/state")
            .dir(".tool/cache/sub")
            .build()
            .unwrap();

        assert!(root.join(".tool/state").is_dir(), "state dir should exist");
        assert!(
            root.join(".tool/cache/sub").is_dir(),
            "nested cache dir should exist"
        );
        assert_eq!(result.created.len(), 2, "both dirs should be created");
    }

    #[test]
    fn test_skip_existing_directory() {
        let dir = tmp();
        let root = project_root(&dir);
        std::fs::create_dir_all(root.join(".tool")).unwrap();

        let result = Scaffold::new(&root).dir(".tool").build().unwrap();

        assert!(
            result.created.is_empty(),
            "existing dir should not be recreated"
        );
        assert_eq!(result.existed.len(), 1, "existing dir should be in existed");
    }

    #[test]
    fn test_write_default_config() {
        let dir = tmp();
        let root = project_root(&dir);

        let result = Scaffold::new(&root)
            .default_config("config.toml", "key = \"value\"\n")
            .build()
            .unwrap();

        let config_path = root.join("config.toml");
        assert!(config_path.is_file(), "config file should exist");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "key = \"value\"\n");
        assert_eq!(result.created.len(), 1);
    }

    #[test]
    fn test_skip_existing_config() {
        let dir = tmp();
        let root = project_root(&dir);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("config.toml"), "original").unwrap();

        let result = Scaffold::new(&root)
            .default_config("config.toml", "new content")
            .build()
            .unwrap();

        // Content should not be overwritten
        let content = std::fs::read_to_string(root.join("config.toml")).unwrap();
        assert_eq!(
            content, "original",
            "existing config should not be overwritten"
        );
        assert!(result.created.is_empty(), "nothing should be created");
        assert_eq!(
            result.existed.len(),
            1,
            "existing file should be in existed"
        );
    }

    #[test]
    fn test_create_nested_config_creates_parent_dir() {
        let dir = tmp();
        let root = project_root(&dir);

        let result = Scaffold::new(&root)
            .default_config("deep/nested/config.toml", "data")
            .build()
            .unwrap();

        assert!(root.join("deep/nested/config.toml").is_file());
        assert_eq!(result.created.len(), 1);
    }

    // ── .gitignore ────────────────────────────────────────────────────

    #[test]
    fn test_gitignore_entry_new_file() {
        let dir = tmp();
        let root = project_root(&dir);

        Scaffold::new(&root)
            .gitignore_entry(".tool/cache/")
            .build()
            .unwrap();

        let gitignore = root.join(".gitignore");
        assert!(gitignore.is_file(), ".gitignore should be created");
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(
            content.contains(".tool/cache/"),
            "gitignore should contain entry"
        );
    }

    #[test]
    fn test_gitignore_append_to_existing() {
        let dir = tmp();
        let root = project_root(&dir);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();

        Scaffold::new(&root)
            .gitignore_entry(".tool/cache/")
            .gitignore_entry("target/")
            .build()
            .unwrap();

        let content = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(
            content.contains("node_modules/"),
            "original content preserved"
        );
        assert!(content.contains(".tool/cache/"), "new entry added");
        assert!(content.contains("target/"), "second new entry added");
    }

    #[test]
    fn test_gitignore_skip_duplicate() {
        let dir = tmp();
        let root = project_root(&dir);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), ".tool/cache/\n").unwrap();

        Scaffold::new(&root)
            .gitignore_entry(".tool/cache/")
            .build()
            .unwrap();

        let content = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        // Should only appear once
        assert_eq!(content.matches(".tool/cache/").count(), 1);
    }

    // ── Managed block ─────────────────────────────────────────────────

    #[test]
    fn test_managed_block() {
        let dir = tmp();
        let root = project_root(&dir);

        Scaffold::new(&root)
            .managed_block("MYTOOL", "config.toml", "# My Tool Config\nkey = \"value\"")
            .build()
            .unwrap();

        let content = std::fs::read_to_string(root.join("config.toml")).unwrap();
        assert!(content.contains("<!-- MYTOOL:START -->"));
        assert!(content.contains("<!-- MYTOOL:END -->"));
        assert!(content.contains("My Tool Config"));
    }

    // ── Agent command file ────────────────────────────────────────────

    #[test]
    fn test_agent_command_file_creates_parent_dir() {
        let dir = tmp();
        let root = project_root(&dir);

        Scaffold::new(&root)
            .agent_command_file(".claude/commands/my-tool.md", "# My Tool\n\nUsage")
            .build()
            .unwrap();

        let path = root.join(".claude/commands/my-tool.md");
        assert!(path.is_file(), "agent command file should exist");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "# My Tool\n\nUsage");
    }

    #[test]
    fn test_multiple_operations() {
        let dir = tmp();
        let root = project_root(&dir);

        let result = Scaffold::new(&root)
            .dir(".tool/state")
            .dir(".tool/cache")
            .default_config("config.toml", "enabled = true\n")
            .gitignore_entry(".tool/cache/")
            .agent_command_file(".claude/commands/my-tool.md", "# command")
            .build()
            .unwrap();

        assert!(result.created.len() >= 4, "should create multiple items");
        assert!(root.join(".tool/state").is_dir());
        assert!(root.join(".tool/cache").is_dir());
        assert!(root.join("config.toml").is_file());
        assert!(root.join(".claude/commands/my-tool.md").is_file());
        assert!(root.join(".gitignore").is_file());
    }
}
