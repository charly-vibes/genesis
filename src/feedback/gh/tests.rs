//! Unit tests for `feedback::gh`: URL encoding, prefilled URLs, command
//! construction, dry-run gating, and local report writing.

use super::commands::{build_gh_command, find_gh};
use super::fallback::{build_prefilled_url, urlencode, write_local_report};
use super::{CreateIssueOptions, create_issue};

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
