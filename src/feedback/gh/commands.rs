//! `gh` command construction and execution.
//!
//! ## Responsibilities
//!
//! - Locate the `gh` binary on `PATH` and check authentication
//! - Build the `gh issue create` argv (execution and display forms)
//! - Spawn `gh` with the body piped on stdin and parse its output
//!
//! ## Rationale
//!
//! Split from `mod.rs` so each step of the invocation pipeline stays under
//! the R8 profile function-size limits.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::{CreateIssueOptions, GhResult};

/// Message describing what `gh issue create` would run (dry-run mode).
pub(super) fn dry_run_message(opts: &CreateIssueOptions) -> String {
    format!(
        "DRY RUN: would run: {} {}",
        build_gh_command(opts),
        format_gh_args(opts)
    )
}

/// Build the `gh issue create` command.
pub(super) fn build_gh_command(opts: &CreateIssueOptions) -> String {
    let mut cmd = format!("gh issue create --repo {}", opts.repo);
    cmd.push_str(&format!(" --title {:?}", opts.title));
    for label in &opts.labels {
        cmd.push_str(&format!(" --label {:?}", label));
    }
    cmd.push_str(" --body-file -");
    cmd
}

/// Format the gh args for display.
fn format_gh_args(opts: &CreateIssueOptions) -> String {
    let labels: Vec<String> = opts
        .labels
        .iter()
        .map(|l| format!("--label '{}'", l))
        .collect();
    format!(
        "--repo '{}' --title '{}' {} --body-file -",
        opts.repo,
        opts.title.replace('\'', "'\\''"),
        labels.join(" ")
    )
}

/// Try to create an issue with `gh`, returning the result or an error string.
pub(super) fn try_create_issue(
    gh_path: &str,
    opts: &CreateIssueOptions,
) -> Result<GhResult, String> {
    let output = run_gh(gh_path, opts)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    parse_created_issue(&String::from_utf8_lossy(&output.stdout))
}

/// Build the `gh issue create` `Command` for execution.
fn build_command(gh_path: &str, opts: &CreateIssueOptions) -> Command {
    let mut cmd = Command::new(gh_path);
    cmd.args(["issue", "create", "--repo", &opts.repo]);
    cmd.args(["--title", &opts.title]);
    cmd.arg("--body-file");
    cmd.arg("-");
    for label in &opts.labels {
        cmd.args(["--label", label]);
    }
    cmd
}

/// Run `gh issue create` with the body piped on stdin.
fn run_gh(gh_path: &str, opts: &CreateIssueOptions) -> Result<std::process::Output, String> {
    let mut cmd = build_command(gh_path, opts);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn gh: {}", e))?;
    write_body_stdin(&mut child, opts);
    child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for gh: {}", e))
}

/// Write the issue body to the spawned `gh` process's stdin (best-effort).
fn write_body_stdin(child: &mut std::process::Child, opts: &CreateIssueOptions) {
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(opts.body.as_bytes());
    }
}

/// Parse the issue URL and number from `gh` stdout
/// (`https://github.com/owner/repo/issues/123`).
fn parse_created_issue(stdout: &str) -> Result<GhResult, String> {
    let url = stdout.trim().to_string();
    let number = url
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Ok(GhResult::Created { url, number })
}

/// Find the `gh` binary on PATH.
pub(super) fn find_gh() -> Option<String> {
    std::env::var("PATH").ok().and_then(|path| {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("gh");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
            #[cfg(windows)]
            {
                let candidate_exe = PathBuf::from(dir).join("gh.exe");
                if candidate_exe.exists() {
                    return Some(candidate_exe.to_string_lossy().to_string());
                }
            }
        }
        None
    })
}

/// Check if `gh` is authenticated.
pub(super) fn is_gh_authenticated(gh_path: &str) -> bool {
    Command::new(gh_path)
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
