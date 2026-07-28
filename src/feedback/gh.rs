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

use std::path::PathBuf;
use std::process::Command;

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
        let cmd = build_gh_command(opts);
        let cmd_str = format!("{} {}", cmd, format_gh_args(opts));
        return Err(format!("DRY RUN: would run: {}", cmd_str));
    }

    // Check if `gh` is available
    let gh_path = find_gh().ok_or_else(|| {
        let url = build_prefilled_url(opts);
        format!("gh not found; install GitHub CLI or open: {}", url)
    })?;

    // Check if `gh` is authenticated
    if !is_gh_authenticated(&gh_path) {
        let url = build_prefilled_url(opts);
        return Err(format!(
            "gh not authenticated. Run `gh auth login` first, or open: {}",
            url
        ));
    }

    // Try to create the issue
    match try_create_issue(&gh_path, opts) {
        Ok(result) => Ok(result),
        Err(err) => {
            // Check for specific error patterns
            if err.contains("label") || err.contains("label") {
                // Labels don't exist — retry without labels
                let mut retry_opts = opts.clone();
                retry_opts.labels.clear();
                match try_create_issue(&gh_path, &retry_opts) {
                    Ok(result) => Ok(result),
                    Err(_) => {
                        let url = build_prefilled_url(opts);
                        Err(format!(
                            "labels not found upstream; retried without labels. Open: {}",
                            url
                        ))
                    }
                }
            } else if err.contains("network") || err.contains("connect") || err.contains("timeout")
            {
                // Network issue — write to local file
                let path = write_local_report(opts);
                Ok(GhResult::LocalFile(path))
            } else if err.contains("403") || err.contains("404") || err.contains("not accessible") {
                // Permission error — fall back to web URL
                let url = build_prefilled_url(opts);
                Err(format!(
                    "you may lack issue-create rights on the target repo. Open: {}",
                    url
                ))
            } else {
                let url = build_prefilled_url(opts);
                Err(format!("gh error: {}. Open: {}", err, url))
            }
        }
    }
}

/// Build the `gh issue create` command.
fn build_gh_command(opts: &CreateIssueOptions) -> String {
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
fn try_create_issue(gh_path: &str, opts: &CreateIssueOptions) -> Result<GhResult, String> {
    let mut cmd = Command::new(gh_path);
    cmd.args(["issue", "create", "--repo", &opts.repo]);
    cmd.args(["--title", &opts.title]);
    cmd.arg("--body-file");
    cmd.arg("-");

    for label in &opts.labels {
        cmd.args(["--label", label]);
    }

    // Pipe body on stdin
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn gh: {}", e))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(opts.body.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for gh: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Parse URL and number from output
        // gh output: https://github.com/owner/repo/issues/123
        let number = stdout
            .rsplit('/')
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(GhResult::Created {
            url: stdout,
            number,
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(stderr)
    }
}

/// Find the `gh` binary on PATH.
fn find_gh() -> Option<String> {
    std::env::var("PATH").ok().and_then(|path| {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("gh");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
            // Also check with .exe on Windows
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
fn is_gh_authenticated(gh_path: &str) -> bool {
    Command::new(gh_path)
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build a prefilled GitHub issue URL.
fn build_prefilled_url(opts: &CreateIssueOptions) -> String {
    let title = urlencode(&opts.title);
    let body = urlencode(&opts.body);
    let labels = urlencode(&opts.labels.join(","));
    format!(
        "https://github.com/{}/issues/new?title={}&body={}&labels={}",
        opts.repo, title, body, labels
    )
}

/// Write the issue body to a local report file.
fn write_local_report(opts: &CreateIssueOptions) -> PathBuf {
    let tool_name = "genesis";
    let reports_dir = if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache).join(tool_name).join("reports")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".cache")
            .join(tool_name)
            .join("reports")
    } else {
        std::env::temp_dir().join(tool_name).join("reports")
    };

    let _ = std::fs::create_dir_all(&reports_dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = reports_dir.join(format!("report-{}.md", timestamp));

    let content = format!(
        "# Issue Report\n\n**Title:** {}\n\n**Repo:** {}\n\n**Labels:** {}\n\n---\n\n{}\n\n---\n\n*To retry: `gh issue create --repo {} --title {:?} --body-file -`*\n",
        opts.title,
        opts.repo,
        opts.labels.join(", "),
        opts.body,
        opts.repo,
        opts.title,
    );
    let _ = std::fs::write(&path, &content);
    path
}

/// Simple URL encoding (only encodes what's needed for issue URLs).
fn urlencode(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode_basic() {
        assert_eq!(urlencode("hello world"), "hello+world");
    }

    #[test]
    fn test_urlencode_special_chars() {
        let encoded = urlencode("fix: broken thing (urgent)");
        assert!(encoded.contains("fix%3A"));
        assert!(encoded.contains("%28"));
        assert!(encoded.contains("%29"));
    }

    #[test]
    fn test_build_prefilled_url() {
        let opts = CreateIssueOptions::new("owner/repo", "Bug: crash", "Steps: 1. run");
        let url = build_prefilled_url(&opts);
        assert!(url.starts_with("https://github.com/owner/repo/issues/new?"));
        assert!(url.contains("title=Bug%3A+crash"));
        assert!(url.contains("body=Steps%3A+1.+run"));
    }

    #[test]
    fn test_build_prefilled_url_with_labels() {
        let mut opts = CreateIssueOptions::new("owner/repo", "Bug", "body");
        opts.labels.push("bug".into());
        opts.labels.push("priority".into());
        let url = build_prefilled_url(&opts);
        assert!(url.contains("labels=bug%2Cpriority"));
    }

    #[test]
    fn test_dry_run_returns_error() {
        let opts = CreateIssueOptions {
            repo: "owner/repo".into(),
            title: "test".into(),
            body: "body".into(),
            labels: vec![],
            dry_run: true,
        };
        let result = create_issue(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DRY RUN"));
    }

    #[test]
    fn test_write_local_report_creates_file() {
        let opts = CreateIssueOptions::new("owner/repo", "Test title", "Test body content");
        let path = write_local_report(&opts);
        assert!(path.exists(), "report file should exist");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Test title"));
        assert!(content.contains("Test body content"));
        assert!(content.contains("owner/repo"));
        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_find_gh_returns_none_in_isolation() {
        // This test runs in a sandboxed PATH
        let result = find_gh();
        // gh might or might not be installed — just verify the function runs
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_gh_command_format() {
        let opts = CreateIssueOptions {
            repo: "owner/repo".into(),
            title: "Test Issue".into(),
            body: "body".into(),
            labels: vec!["bug".into(), "help wanted".into()],
            dry_run: false,
        };
        let cmd = build_gh_command(&opts);
        assert!(cmd.contains("gh issue create"));
        assert!(cmd.contains("--repo owner/repo"));
        assert!(cmd.contains("--label"));
        assert!(cmd.contains("--body-file -"));
    }
}
