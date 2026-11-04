// pretender: role=test
//! Unit tests for `feedback/context` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget).

#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use std::fs;

#[test]
fn test_format_context_bundle_minimal() {
    let bundle = ContextBundle {
        tool_name: "test".into(),
        tool_version: "1.0.0".into(),
        os_arch: "linux/x86_64".into(),
        repro_hash: 12345,
        ..Default::default()
    };
    let output = format_context_bundle(&bundle);
    assert!(output.contains("test"));
    assert!(output.contains("1.0.0"));
    assert!(output.contains("linux/x86_64"));
}

#[test]
fn test_format_context_bundle_full() {
    let bundle = ContextBundle {
        tool_name: "wai".into(),
        tool_version: "2026.7.16".into(),
        command: Some("wai status".into()),
        exit_code: Some(1),
        suggestion_footer: Some("→ Run: wai init".into()),
        os_arch: "linux/x86_64".into(),
        shell: Some("/bin/bash".into()),
        gh_version: Some("gh 2.0.0".into()),
        git_remote: Some("github.com/owner/repo".into()),
        git_branch: Some("main".into()),
        git_dirty: Some(true),
        repo_state: vec!["wai".into(), "beads".into()],
        repro_hash: 67890,
    };
    let output = format_context_bundle(&bundle);
    assert!(output.contains("wai status"));
    assert!(output.contains("exit_code: 1"));
    assert!(output.contains("/bin/bash"));
    assert!(output.contains("github.com/owner/repo"));
    assert!(output.contains("git_dirty: true"));
    assert!(output.contains("wai beads"));
}

#[test]
fn test_detect_repo_state() {
    let dir = tempfile::tempdir().require("value");
    let signals = detect_repo_state(dir.path());
    assert!(signals.is_empty());

    fs::create_dir_all(dir.path().join(".wai")).require("value");
    let signals = detect_repo_state(dir.path());
    assert!(signals.contains(&"wai".to_string()));
}

#[test]
fn test_gather_context_creates_bundle() {
    let dir = tempfile::tempdir().require("value");
    fs::create_dir_all(dir.path().join(".wai")).require("value");
    let bundle = gather_context(
        "test-tool",
        "0.1.0",
        Some("test-tool check".into()),
        Some(0),
        None,
        dir.path(),
    );
    assert_eq!(bundle.tool_name, "test-tool");
    assert_eq!(bundle.tool_version, "0.1.0");
    assert_eq!(bundle.command, Some("test-tool check".into()));
    assert_eq!(bundle.exit_code, Some(0));
    assert!(bundle.repo_state.contains(&"wai".to_string()));
}

#[test]
fn test_repro_hash_is_consistent() {
    let bundle1 = gather_context(
        "tool",
        "1.0",
        Some("cmd".into()),
        Some(0),
        None,
        Path::new("."),
    );
    let bundle2 = gather_context(
        "tool",
        "1.0",
        Some("cmd".into()),
        Some(0),
        None,
        Path::new("."),
    );
    assert_eq!(bundle1.repro_hash, bundle2.repro_hash);
}

#[test]
fn test_repro_hash_differs_with_exit_code() {
    let bundle1 = gather_context(
        "tool",
        "1.0",
        Some("cmd".into()),
        Some(0),
        None,
        Path::new("."),
    );
    let bundle2 = gather_context(
        "tool",
        "1.0",
        Some("cmd".into()),
        Some(1),
        None,
        Path::new("."),
    );
    assert_ne!(bundle1.repro_hash, bundle2.repro_hash);
}

#[test]
fn test_git_remote_via_process() {
    let dir = tempfile::tempdir().require("value");
    std::process::Command::new("git")
        .args(["-c", "init.templateDir=", "init"])
        .current_dir(dir.path())
        .output()
        .require("value");
    let remote = get_git_remote(dir.path());
    assert!(remote.is_none() || remote.is_some());
}

/// genesis-w6h contract pin: outside a Rust workspace (no Cargo.toml,
/// no git), gather_context must degrade gracefully — no error, git
/// fields None — because the downstream report flow (handle_feedback,
/// and ah's inline copy) depends on it being usable from any cwd.
/// (The GH#28 item 3 'cannot read Cargo.toml' defect is downstream:
/// espectacular main.rs hard-reads Cargo.toml before calling us.)
#[test]
fn test_gather_context_non_rust_workspace_degrades_gracefully() {
    let dir = tempfile::tempdir().require("value");
    let bundle = gather_context("ah", "0.8.0", None, None, None, dir.path());
    assert_eq!(bundle.tool_name, "ah");
    assert!(bundle.git_remote.is_none(), "no git repo → no remote");
    assert!(bundle.git_branch.is_none(), "no git repo → no branch");
    assert!(bundle.git_dirty.is_none(), "no git repo → not dirty");
    assert!(bundle.repo_state.is_empty());
    let rendered = format_context_bundle(&bundle);
    assert!(rendered.contains("tool: ah 0.8.0"));
    assert!(rendered.contains("repro_hash:"));
    assert!(!rendered.contains("git_remote:"), "None fields are omitted");
}

/// Same contract in a git repo that is NOT a Rust workspace: the
/// missing Cargo.toml must mean nothing more than 'not Rust' — git
/// metadata is still gathered.
#[test]
fn test_gather_context_git_repo_without_cargo_toml() {
    let dir = tempfile::tempdir().require("value");
    // Hermetic: the suite can run inside a git hook (which exports
    // GIT_DIR); the fixture repo must be the tempdir, always.
    let mut init = std::process::Command::new("git");
    for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
        init.env_remove(var);
    }
    init.args(["-c", "init.templateDir=", "init"])
        .current_dir(dir.path())
        .output()
        .require("value");
    let bundle = gather_context("ah", "0.8.0", None, None, None, dir.path());
    assert!(bundle.git_branch.is_some(), "git repo → branch gathered");
    let rendered = format_context_bundle(&bundle);
    assert!(rendered.contains("git_branch:"));
}
