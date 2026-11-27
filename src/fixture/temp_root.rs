//! The in-progress fixture root guard: an owned temp directory that is
//! deleted on drop unless a successful build disarms it.

use std::path::{Path, PathBuf};

use super::FixtureError;

/// Create the fixture root as an owned temp directory.
///
/// `TempDir::keep()` consumes the `TempDir`, leaving the directory on disk.
/// While the build is still in progress the path is held by a
/// [`TempRoot`] guard that deletes it on drop, so any later `?` in
/// [`super::FixtureBuilder::build`] cannot leak the temp root; a successful
/// build disarms the guard and hands the path to [`super::Fixture`].
pub(super) fn create_root() -> Result<TempRoot, FixtureError> {
    tempfile::tempdir()
        .map_err(|e| FixtureError::Io {
            path: PathBuf::from("<tempdir>"),
            message: e.to_string(),
        })
        .map(|dir| TempRoot(dir.keep()))
}

/// An in-progress fixture root, deleted on drop unless disarmed.
///
/// A successful build calls [`TempRoot::into_path`], which disarms the
/// guard and transfers deletion responsibility to [`super::Fixture::drop`].
#[derive(Debug)]
pub(super) struct TempRoot(PathBuf);

impl TempRoot {
    /// Disarm the guard and hand the root path to the caller.
    pub(super) fn into_path(self) -> PathBuf {
        let root = self.0.clone();
        std::mem::forget(self);
        root
    }
}

impl std::ops::Deref for TempRoot {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
