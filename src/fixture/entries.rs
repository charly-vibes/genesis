//! Builder entry types and their build-time materializers.

use std::path::Path;

use super::FixtureError;

/// A file entry in the builder.
#[derive(Debug)]
pub(super) struct FileEntry {
    pub(super) path: String,
    pub(super) content: String,
}

/// A TOML file entry in the builder.
#[derive(Debug)]
pub(super) struct TomlEntry {
    pub(super) path: String,
    pub(super) value: toml::Value,
}

/// A deferred config write captured at `with_config` time.
///
/// Stored as a closure so the typed config is written to `ConfigFile::path()`
/// at `build()` time, once the fixture root exists.
type ConfigWriter = Box<dyn FnOnce(&Path) -> Result<(), FixtureError>>;

pub(super) struct ConfigEntry {
    pub(super) write: ConfigWriter,
}

impl std::fmt::Debug for ConfigEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigEntry").finish_non_exhaustive()
    }
}

/// Create the requested marker directories under `root`.
pub(super) fn create_markers(root: &Path, markers: &[String]) -> Result<(), FixtureError> {
    for marker in markers {
        let path = root.join(marker);
        std::fs::create_dir_all(&path).map_err(|e| FixtureError::Io {
            path: path.clone(),
            message: format!("create marker '{marker}': {e}"),
        })?;
    }
    Ok(())
}

/// Create the parent directory of `path` (no-op when there is none).
pub(super) fn create_parent(path: &Path, label: &str) -> Result<(), FixtureError> {
    match path.parent() {
        Some(parent) => std::fs::create_dir_all(parent).map_err(|e| FixtureError::Io {
            path: parent.to_path_buf(),
            message: format!("create parent for '{label}': {e}"),
        }),
        None => Ok(()),
    }
}

/// Write the plain (already-serialized) files under `root`.
pub(super) fn write_plain_files(root: &Path, files: &[FileEntry]) -> Result<(), FixtureError> {
    for entry in files {
        let path = root.join(&entry.path);
        create_parent(&path, &entry.path)?;
        std::fs::write(&path, &entry.content).map_err(|e| FixtureError::Io {
            path: path.clone(),
            message: format!("write file '{}': {e}", entry.path),
        })?;
    }
    Ok(())
}

/// Serialize and write the TOML files under `root`.
pub(super) fn write_toml_files(root: &Path, tomls: &[TomlEntry]) -> Result<(), FixtureError> {
    for entry in tomls {
        let path = root.join(&entry.path);
        create_parent(&path, &entry.path)?;
        let content = toml::to_string_pretty(&entry.value).map_err(|e| FixtureError::Io {
            path: path.clone(),
            message: format!("serialize TOML for '{}': {e}", entry.path),
        })?;
        std::fs::write(&path, &content).map_err(|e| FixtureError::Io {
            path: path.clone(),
            message: format!("write TOML file '{}': {e}", entry.path),
        })?;
    }
    Ok(())
}
