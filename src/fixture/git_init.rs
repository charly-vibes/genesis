//! Deterministic `git init` seeding for fixtures.

use std::path::Path;
use std::process::Command;

use super::FixtureError;
use crate::fixture::command::hermetic_git;

/// Run `git init` plus an initial commit in `root`.
pub(super) fn git_init_repo(root: &Path) -> Result<(), FixtureError> {
    let run = |stage: &'static str, args: &[&str]| -> Result<std::process::Output, FixtureError> {
        hermetic_git(&mut Command::new("git"))
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| FixtureError::Git { stage, source: e })
    };

    require_success(run("init", &["init"]), "init")?;
    seed_gitkeep(root)?;
    seed_identity(root);
    require_success(run("add", &["add", "-A"]), "add")?;
    let commit = hermetic_git(&mut Command::new("git"))
        .args(["commit", "-m", "initial"])
        .current_dir(root)
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
        .output()
        .map_err(|e| FixtureError::Git {
            stage: "commit",
            source: e,
        })?;
    require_success(Ok(commit), "commit")?;
    Ok(())
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
    let configure = |key: &str, value: &str| -> Result<std::process::Output, FixtureError> {
        hermetic_git(&mut Command::new("git"))
            .args(["config", key, value])
            .current_dir(root)
            .output()
            .map_err(|e| FixtureError::Git {
                stage: "config",
                source: e,
            })
    };
    let _ = configure("user.email", "test@test.com");
    let _ = configure("user.name", "Test");
}

/// Map a non-zero git exit to a [`FixtureError::Git`] with context.
fn require_success(
    output: Result<std::process::Output, FixtureError>,
    stage: &'static str,
) -> Result<std::process::Output, FixtureError> {
    let output = output?;
    if output.status.success() {
        return Ok(output);
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
