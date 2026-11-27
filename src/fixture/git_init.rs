//! Deterministic `git init` seeding for fixtures.
//!
//! Seeded repositories are isolated from the host: both git config scopes
//! read an empty file, the commit is forced unsigned, and the initial
//! branch is pinned to `main`. Whatever the host user configured
//! (`commit.gpgsign`, `init.defaultBranch`, `core.hooksPath`, identity,
//! …) cannot change what a fixture contains.

use std::path::Path;
use std::process::Command;

use super::FixtureError;
use crate::fixture::command::hermetic_git;

/// Author/committer date pinned so fixture history stays reproducible.
const COMMIT_DATE: &str = "2020-01-01T00:00:00Z";

/// Deterministic branch name for seeded repositories.
const INITIAL_BRANCH: &str = "main";

/// Empty config file: host global/system git config (commit.gpgsign,
/// init.defaultBranch, core.hooksPath, …) must not alter seeded repos.
/// A missing file is ignored by git, so this also reads as empty config on
/// platforms without `/dev/null`.
const EMPTY_CONFIG: &str = "/dev/null";

/// Environment pointing both git config scopes at [`EMPTY_CONFIG`].
const ISOLATED_CONFIG_ENVS: [(&str, &str); 2] = [
    ("GIT_CONFIG_GLOBAL", EMPTY_CONFIG),
    ("GIT_CONFIG_SYSTEM", EMPTY_CONFIG),
];

/// Run `git init` plus an initial commit in `root`.
///
/// Pins the initial branch: `-b` is supported by git 2.28+; older
/// releases fall back to the `init.defaultBranch` config override.
pub(super) fn git_init_repo(root: &Path) -> Result<(), FixtureError> {
    if git(root, "init", &["init", "-b", INITIAL_BRANCH], &[]).is_err() {
        let fallback = format!("init.defaultBranch={INITIAL_BRANCH}");
        git(root, "init", &["init", "-c", &fallback], &[])?;
    }
    seed_gitkeep(root)?;
    seed_identity(root);
    git(root, "add", &["add", "-A"], &[])?;
    git(
        root,
        "commit",
        &["-c", "commit.gpgsign=false", "commit", "-m", "initial"],
        &[
            ("GIT_AUTHOR_DATE", COMMIT_DATE),
            ("GIT_COMMITTER_DATE", COMMIT_DATE),
        ],
    )?;
    Ok(())
}

/// Run a git subcommand in `root`, hermetically; spawn or non-zero exit
/// failure maps to [`FixtureError::Git`] with `stage` context.
///
/// Every invocation also gets [`ISOLATED_CONFIG_ENVS`] so host global and
/// system git config cannot alter fixture repositories.
fn git(
    root: &Path,
    stage: &'static str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<(), FixtureError> {
    let output = hermetic_git(&mut Command::new("git"))
        .args(args)
        .current_dir(root)
        .envs(ISOLATED_CONFIG_ENVS)
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
