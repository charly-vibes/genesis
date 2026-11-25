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
        let (mut created, mut existed) = scaffold_dirs(&self)?;
        let (more_created, more_existed) = scaffold_files(&self)?;
        created.extend(more_created);
        existed.extend(more_existed);
        let (gitignore_created, gitignore_existed) = update_gitignore(&self)?;
        created.extend(gitignore_created);
        existed.extend(gitignore_existed);

        Ok(ScaffoldResult { created, existed })
    }
}

/// Create the builder's directories; existing ones count as "existed".
fn scaffold_dirs(builder: &Scaffold) -> std::io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let paths: Vec<PathBuf> = builder
        .dirs
        .iter()
        .map(|dir| builder.project_root.join(dir))
        .collect();
    let pre_existing: Vec<bool> = paths.iter().map(|p| p.exists()).collect();
    for (path, existed_before) in paths.iter().zip(&pre_existing) {
        if !existed_before {
            std::fs::create_dir_all(path)?;
        }
    }
    let existed = partition_existing(paths.clone(), &pre_existing);
    let created = partition_new(paths, &pre_existing);
    Ok((created, existed))
}

/// Write the builder's files (no overwrite); existing ones count as "existed".
fn scaffold_files(builder: &Scaffold) -> std::io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let paths: Vec<PathBuf> = builder
        .files
        .iter()
        .map(|(rel_path, _)| builder.project_root.join(rel_path))
        .collect();
    let pre_existing: Vec<bool> = paths.iter().map(|p| p.exists()).collect();
    for ((path, existed_before), (_rel_path, content)) in
        paths.iter().zip(&pre_existing).zip(&builder.files)
    {
        if !existed_before {
            write_new_file(path, content).map_err(|e| annotate(e, &path.display().to_string()))?;
        }
    }
    let existed = partition_existing(paths.clone(), &pre_existing);
    let created = partition_new(paths, &pre_existing);
    Ok((created, existed))
}

/// Wrap an io error with the file that caused it.
fn annotate(err: std::io::Error, rel_path: &str) -> std::io::Error {
    std::io::Error::other(format!("{rel_path}: {err}"))
}

/// Paths that already existed before the run.
fn partition_existing(paths: Vec<PathBuf>, pre_existing: &[bool]) -> Vec<PathBuf> {
    paths
        .into_iter()
        .zip(pre_existing)
        .filter(|(_, existed)| **existed)
        .map(|(path, _)| path)
        .collect()
}

/// Paths created by this run.
fn partition_new(paths: Vec<PathBuf>, pre_existing: &[bool]) -> Vec<PathBuf> {
    paths
        .into_iter()
        .zip(pre_existing)
        .filter(|(_, existed)| !**existed)
        .map(|(path, _)| path)
        .collect()
}

/// Write `content` to `abs_path`, creating parent directories on demand.
fn write_new_file(abs_path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = abs_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(abs_path, content)
}

/// Merge the builder's `.gitignore` entries into the project's `.gitignore`.
///
/// Returns `(created, existed)` paths according to whether the file was
/// newly created, rewritten, or left untouched.
fn update_gitignore(builder: &Scaffold) -> std::io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    if builder.gitignore_entries.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let gitignore_path = builder.project_root.join(".gitignore");
    let gitignore_existed = gitignore_path.exists();
    let wrote_entries = merge_gitignore_entries(
        &gitignore_path,
        gitignore_existed,
        &builder.gitignore_entries,
    )?;
    Ok(classify_gitignore(
        gitignore_path,
        gitignore_existed,
        wrote_entries,
    ))
}

/// Append the builder's missing entries to `.gitignore`, creating parent
/// directories on demand; returns whether any entry was written.
fn merge_gitignore_entries(
    gitignore_path: &Path,
    existed: bool,
    entries: &[String],
) -> std::io::Result<bool> {
    if let Some(parent) = gitignore_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut existing = read_gitignore(gitignore_path, existed)?;
    let new_entries = missing_entries(&existing, entries);
    if new_entries.is_empty() {
        return Ok(false);
    }
    existing.push_str(&new_entries.concat());
    std::fs::write(gitignore_path, &existing)?;
    Ok(true)
}

/// Record the `.gitignore` path as created/existed. An absent file with no
/// new entries is still kept (now present on disk, empty).
fn classify_gitignore(
    gitignore_path: PathBuf,
    gitignore_existed: bool,
    wrote_entries: bool,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    if gitignore_existed || wrote_entries {
        (Vec::new(), vec![gitignore_path])
    } else {
        (vec![gitignore_path], Vec::new())
    }
}

/// Current `.gitignore` content, newline-terminated; empty when absent.
fn read_gitignore(gitignore_path: &Path, existed: bool) -> std::io::Result<String> {
    if !existed {
        return Ok(String::new());
    }
    let mut prior = std::fs::read_to_string(gitignore_path)?;
    if !prior.ends_with('\n') {
        prior.push('\n');
    }
    Ok(prior)
}

/// Entries missing from the current content, newline-terminated.
fn missing_entries(existing: &str, entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| format!("{}\n", entry))
        .filter(|line| !existing.contains(line.as_str()) && !existing.contains(line.trim()))
        .collect()
}

#[cfg(test)]
#[path = "scaffold/tests.rs"]
mod tests;
