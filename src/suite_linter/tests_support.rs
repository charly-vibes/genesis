//! Shared fixtures for the `suite_linter` test modules: a fixed-output mock
//! check and a scratch repo root outside any git worktree.

// pretender: role=test
use crate::suite_linter::{LintCheck, LintResult};

use std::path::{Path, PathBuf};

/// One parameterized mock check replaces five structurally identical mock
/// impls (each with name/description/run methods) — pretender flags
/// clusters of identical-shaped units.
pub(super) struct FixedCheck {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) results: Vec<LintResult>,
}

impl LintCheck for FixedCheck {
    fn name(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.name == "panic" {
            return Err("internal failure".into());
        }
        Ok(self.results.clone())
    }
}

/// Scratch root that must not live inside a git repository.
pub(super) fn test_root() -> PathBuf {
    let root = std::env::temp_dir().join("genesis-suite-linter-test");
    assert!(
        !root.join(".git").exists(),
        "test root must not live inside a git repository"
    );
    root
}
