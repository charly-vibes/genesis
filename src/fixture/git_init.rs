//! Deterministic `git init` seeding for fixtures.

use std::path::Path;
use std::process::Command;

use super::FixtureError;
use crate::fixture::command::hermetic_git;

/// Author/committer date pinned so fixture history stays reproducible.
const COMMIT_DATE: &str = "2020-01-01T00:00:00Z";

/// Run `git init` plus an initial commit in `root`.
pub(super) fn git_init_repo(root: &Path) -> Result<(), FixtureError> {
    git(root, "init", &["init"], &[])?;
    seed_gitkeep(root)?;
    seed_identity(root);
    git(root, "add", &["add", "-A"], &[])?;
    git(
        root,
        "commit",
        &["commit", "-m", "initial"],
        &[
            ("GIT_AUTHOR_DATE", COMMIT_DATE),
            ("GIT_COMMITTER_DATE", COMMIT_DATE),
        ],
    )?;
    Ok(())
}

/// Run a git subcommand in `root`, hermetically; spawn or non-zero exit
/// failure maps to [`FixtureError::Git`] with `stage` context.
fn git(
    root: &Path,
    stage: &'static str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<(), FixtureError> {
    let output = hermetic_git(&mut Command::new("git"))
        .args(args)
        .current_dir(root)
        .envs(env.iter().copied())
        .output()
        .map_err(|e| FixtureError::Git { stage, source: e })?;
    require_success(output, stage)
}

/// Seed an empty `.gitkeep` so the initial commit always has content.
fn seed_gitkeep(root: &Path) -> Result<(), FixtureError> {
    std::fs::write(root.join(".gitkeep"), "").map_err(|e| FixtureError::Git {
        stage: "seed",
        source: e,
    })
}

/// Configure a deterministic commit identity (failures ignored: not fatal).
fn seed_identity(root: &Path) {
    let _ = git(
        root,
        "config",
        &["config", "user.email", "test@test.com"],
        &[],
    );
    let _ = git(root, "config", &["config", "user.name", "Test"], &[]);
}

/// Map a non-zero git exit to a [`FixtureError::Git`] with context.
fn require_success(output: std::process::Output, stage: &'static str) -> Result<(), FixtureError> {
    if output.status.success() {
        return Ok(());
    }
    Err(FixtureError::Git {
        stage,
        source: std::io::Error::other(format!(
            "git {stage} exited {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        )),
    })
}
