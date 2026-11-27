//! Fallback ladder for `gh` failures.
//!
//! ## Responsibilities
//!
//! - Classify raw `gh` errors (labels / network / permission / other)
//! - Provide each fallback path: retry without labels, local report file,
//!   prefilled web URL
//!
//! ## Rationale
//!
//! Split from `mod.rs` so every branch stays under the R8 profile
//! function-size limits while the ladder semantics stay in one place.

use std::path::PathBuf;

use super::CreateIssueOptions;
use super::commands::{find_gh, try_create_issue};

/// Error message when the `gh` binary is missing.
pub(super) fn gh_missing_message(opts: &CreateIssueOptions) -> String {
    format!(
        "gh not found; install GitHub CLI or open: {}",
        build_prefilled_url(opts)
    )
}

/// Error message when `gh` is present but not authenticated.
pub(super) fn gh_unauthenticated_message(opts: &CreateIssueOptions) -> String {
    format!(
        "gh not authenticated. Run `gh auth login` first, or open: {}",
        build_prefilled_url(opts)
    )
}

/// Map a raw `gh` failure to a user-facing message with a fallback path:
/// missing labels → retry without them; network issues → local report file;
/// permission errors → prefilled web URL.
pub(super) fn map_gh_error(opts: &CreateIssueOptions, err: &str) -> String {
    if err.contains("label") {
        return retry_without_labels(opts);
    }
    if is_network_error(err) {
        return network_fallback(opts);
    }
    if is_permission_error(err) {
        return permission_fallback(opts);
    }
    format!("gh error: {}. Open: {}", err, build_prefilled_url(opts))
}

/// Does this error look like a network problem?
fn is_network_error(err: &str) -> bool {
    err.contains("network") || err.contains("connect") || err.contains("timeout")
}

/// Does this error look like a repo-permission problem?
fn is_permission_error(err: &str) -> bool {
    err.contains("403") || err.contains("404") || err.contains("not accessible")
}

/// Network failure path: persist the report locally and point the user at it.
fn network_fallback(opts: &CreateIssueOptions) -> String {
    let path = write_local_report(opts);
    format!(
        "network issue; report saved locally at {}. Open later: {}",
        path.display(),
        build_prefilled_url(opts)
    )
}

/// Permission failure path: point the user at the prefilled web URL.
fn permission_fallback(opts: &CreateIssueOptions) -> String {
    format!(
        "you may lack issue-create rights on the target repo. Open: {}",
        build_prefilled_url(opts)
    )
}

/// Retry issue creation without labels (labels may not exist upstream).
fn retry_without_labels(opts: &CreateIssueOptions) -> String {
    let mut retry_opts = opts.clone();
    retry_opts.labels.clear();
    let gh_path = find_gh().unwrap_or_default();
    if try_create_issue(&gh_path, &retry_opts).is_ok() {
        return "created without labels".to_string();
    }
    format!(
        "labels not found upstream; retried without labels. Open: {}",
        build_prefilled_url(opts)
    )
}

/// Build a prefilled GitHub issue URL.
pub(super) fn build_prefilled_url(opts: &CreateIssueOptions) -> String {
    let title = urlencode(&opts.title);
    let body = urlencode(&opts.body);
    let labels = urlencode(&opts.labels.join(","));
    format!(
        "https://github.com/{}/issues/new?title={}&body={}&labels={}",
        opts.repo, title, body, labels
    )
}

/// Write the issue body to a local report file.
pub(super) fn write_local_report(opts: &CreateIssueOptions) -> PathBuf {
    let reports_dir = reports_dir();
    let _ = std::fs::create_dir_all(&reports_dir);
    let path = reports_dir.join(format!("report-{}.md", unix_timestamp_secs()));
    let content = local_report_content(opts);
    let _ = std::fs::write(&path, &content);
    path
}

/// Whole seconds since the Unix epoch (best-effort; 0 on clock failure).
fn unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Render the markdown body of a local fallback report.
fn local_report_content(opts: &CreateIssueOptions) -> String {
    format!(
        "# Issue Report\n\n**Title:** {}\n\n**Repo:** {}\n\n**Labels:** {}\n\n---\n\n{}\n\n---\n\n*To retry: `gh issue create --repo {} --title {:?} --body-file -`*\n",
        opts.title,
        opts.repo,
        opts.labels.join(", "),
        opts.body,
        opts.repo,
        opts.title,
    )
}

/// Simple URL encoding (only encodes what's needed for issue URLs).
pub(super) fn urlencode(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Directory for local fallback reports: `$XDG_CACHE_HOME/genesis/reports`,
/// falling back to `~/.cache` and then the temp dir. Empty values fall
/// through (genesis-4xl): an empty `XDG_CACHE_HOME` must never resolve to a
/// relative path that writes reports into the cwd.
fn reports_dir() -> PathBuf {
    reports_dir_for(
        std::env::var("XDG_CACHE_HOME").ok(),
        std::env::var("HOME").ok(),
    )
}

/// `reports_dir` with injected env values.
fn reports_dir_for(xdg: Option<String>, home: Option<String>) -> PathBuf {
    const TOOL: &str = "genesis";
    if let Some(cache) = xdg.filter(|v| !v.is_empty()) {
        return PathBuf::from(cache).join(TOOL).join("reports");
    }
    if let Some(home) = home.filter(|v| !v.is_empty()) {
        return PathBuf::from(home)
            .join(".cache")
            .join(TOOL)
            .join("reports");
    }
    std::env::temp_dir().join(TOOL).join("reports")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// genesis-4xl: an empty (or unset) `XDG_CACHE_HOME` must fall through
    /// to `$HOME/.cache` or the temp dir — never to a relative path that
    /// would write local fallback reports into the cwd.
    #[test]
    fn reports_dir_empty_xdg_is_never_relative() {
        let home = "/home/report-tester";
        let expected = home_reports_dir(home);
        assert_eq!(
            reports_dir_for(Some(String::new()), Some(home.into())),
            expected
        );
        assert_eq!(reports_dir_for(None, Some(home.into())), expected);
        assert_tmp_reports_fallthrough();
        assert_custom_xdg_wins(home);
    }

    /// `.cache/genesis/reports` under the given HOME.
    fn home_reports_dir(home: &str) -> PathBuf {
        PathBuf::from(home)
            .join(".cache")
            .join("genesis")
            .join("reports")
    }

    /// Empty XDG and empty HOME both fall through to the temp dir, which
    /// must never be relative.
    fn assert_tmp_reports_fallthrough() {
        let tmp_reports = reports_dir_for(Some(String::new()), None);
        assert_eq!(
            tmp_reports,
            std::env::temp_dir().join("genesis").join("reports")
        );
        assert!(
            tmp_reports.is_absolute(),
            "must not be relative: {tmp_reports:?}"
        );
        // Empty HOME also falls through to the temp dir.
        assert_eq!(reports_dir_for(None, Some(String::new())), tmp_reports);
    }

    /// A set, non-empty XDG still wins over HOME-derived paths.
    fn assert_custom_xdg_wins(home: &str) {
        assert_eq!(
            reports_dir_for(Some("/custom/cache".into()), Some(home.into())),
            PathBuf::from("/custom/cache/genesis/reports")
        );
    }
}
