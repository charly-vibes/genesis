//! GitHub CLI invocation with fallback ladder (§7 of agent-issue-reporting.md).
//!
//! ## Primary path
//!
//! `gh issue create --repo <OWNER/REPO> --title "<TITLE>" --body-file - --label "<l1>" ...`
//!
//! ## Fallback ladder
//!
//! 1. `gh` missing → prefilled URL + `Suggestion::Fix`
//! 2. Unauthed → prefilled URL + `gh auth login` hint
//! 3. Labels don't exist → retry without labels
//! 4. No network → write to local file
//! 5. Permission error → prefilled web URL
//!
//! ## Responsibilities
//!
//! - Public API: [`GhResult`], [`CreateIssueOptions`], [`create_issue`]
//! - Gate the ladder on `gh` availability and authentication
//! - Delegate argv construction to `commands` and failure handling to `fallback`

mod commands;
mod fallback;
#[cfg(test)]
mod tests;

use commands::{dry_run_message, find_gh, is_gh_authenticated, try_create_issue};
use fallback::{gh_missing_message, gh_unauthenticated_message, map_gh_error};

use std::path::PathBuf;

/// Result of a `gh issue create` invocation.
#[derive(Debug, Clone)]
pub enum GhResult {
    /// Issue created successfully.
    Created { url: String, number: u64 },
    /// Issue not created; user should use the provided URL.
    FallbackUrl(String),
    /// Issue body written to a local file (network unavailable).
    LocalFile(PathBuf),
}

/// Options for creating an issue.
#[derive(Debug, Clone)]
pub struct CreateIssueOptions {
    /// Target repo in `owner/repo` format.
    pub repo: String,
    /// Issue title.
    pub title: String,
    /// Issue body (markdown).
    pub body: String,
    /// Labels to apply.
    pub labels: Vec<String>,
    /// If true, only print what would be done.
    pub dry_run: bool,
}

impl CreateIssueOptions {
    pub fn new(repo: &str, title: &str, body: &str) -> Self {
        Self {
            repo: repo.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            labels: Vec::new(),
            dry_run: false,
        }
    }
}

/// Create a GitHub issue using `gh`, with fallback ladder.
///
/// Returns the result of the attempt, or an error message describing what
/// went wrong (for inclusion in a `Suggestion::Fix`).
pub fn create_issue(opts: &CreateIssueOptions) -> Result<GhResult, String> {
    if opts.dry_run {
        return Err(dry_run_message(opts));
    }

    // Check if `gh` is available
    let gh_path = find_gh().ok_or_else(|| gh_missing_message(opts))?;

    // Check if `gh` is authenticated
    if !is_gh_authenticated(&gh_path) {
        return Err(gh_unauthenticated_message(opts));
    }

    // Try to create the issue, with typed fallbacks for known failure modes
    try_create_issue(&gh_path, opts).map_err(|err| map_gh_error(opts, &err))
}
