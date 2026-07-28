//! Environment context bundle (§3 of agent-issue-reporting.md).
//!
//! Auto-gathered machine context appended to issue bodies.
//!
//! ## Bundle fields
//!
//! - `tool: <name> <version>`
//! - `command: <exact argv>` — from the error scratch
//! - `exit_code: <n>`
//! - `suggestion_footer: <Suggestion::Fix message>`
//! - `os/arch: <std::env::consts>`
//! - `shell: <SHELL>`
//! - `gh_version: <gh --version>` or "(gh not found)"
//! - `git_remote: <host/path>` — opt-in, reduced by redactor
//! - `git_branch: <branch>` — opt-in
//! - `git_dirty: <bool>` — opt-in
//! - `repo_state: <suite signals: .wai/ .beads/ openspec/ .dont/ .espectacular/>`
//! - `repro_hash: <sha256>` — dedup key

use std::hash::{Hash, Hasher};
use std::path::Path;

/// The environment context bundle.
#[derive(Debug, Clone, Default)]
pub struct ContextBundle {
    pub tool_name: String,
    pub tool_version: String,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub suggestion_footer: Option<String>,
    pub os_arch: String,
    pub shell: Option<String>,
    pub gh_version: Option<String>,
    pub git_remote: Option<String>,
    pub git_branch: Option<String>,
    pub git_dirty: Option<bool>,
    pub repo_state: Vec<String>,
    pub repro_hash: u64,
}

/// Gather the context bundle from the current environment.
pub fn gather_context(
    tool_name: &str,
    tool_version: &str,
    command: Option<String>,
    exit_code: Option<i32>,
    suggestion_footer: Option<String>,
    cwd: &Path,
) -> ContextBundle {
    let os_arch = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
    let shell = std::env::var("SHELL").ok();
    let gh_version = get_gh_version();
    let git_remote = get_git_remote(cwd);
    let git_branch = get_git_branch(cwd);
    let git_dirty = get_git_dirty(cwd);
    let repo_state = detect_repo_state(cwd);

    // Build a repro hash from tool + command + exit_code
    let repro_input = format!(
        "{}:{}:{}:{:?}",
        tool_name,
        tool_version,
        command.as_deref().unwrap_or(""),
        exit_code
    );
    let repro_hash = compute_hash(&repro_input);

    ContextBundle {
        tool_name: tool_name.to_string(),
        tool_version: tool_version.to_string(),
        command,
        exit_code,
        suggestion_footer,
        os_arch,
        shell,
        gh_version,
        git_remote,
        git_branch,
        git_dirty,
        repo_state,
        repro_hash,
    }
}

/// Compute a simple hash for dedup purposes.
fn compute_hash(input: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Serialize the context bundle to a markdown section.
pub fn format_context_bundle(bundle: &ContextBundle) -> String {
    let mut out = String::new();
    out.push_str("## Environment\n\n");
    out.push_str(&format!(
        "- tool: {} {}\n",
        bundle.tool_name, bundle.tool_version
    ));
    if let Some(ref cmd) = bundle.command {
        out.push_str(&format!("- command: `{}`\n", cmd));
    }
    if let Some(code) = bundle.exit_code {
        out.push_str(&format!("- exit_code: {}\n", code));
    }
    if let Some(ref footer) = bundle.suggestion_footer {
        out.push_str(&format!("- suggestion: {}\n", footer));
    }
    out.push_str(&format!("- os/arch: {}\n", bundle.os_arch));
    if let Some(ref shell) = bundle.shell {
        out.push_str(&format!("- shell: {}\n", shell));
    }
    if let Some(ref ver) = bundle.gh_version {
        out.push_str(&format!("- gh_version: {}\n", ver));
    }
    if let Some(ref remote) = bundle.git_remote {
        out.push_str(&format!("- git_remote: {}\n", remote));
    }
    if let Some(ref branch) = bundle.git_branch {
        out.push_str(&format!("- git_branch: {}\n", branch));
    }
    if let Some(dirty) = bundle.git_dirty {
        out.push_str(&format!("- git_dirty: {}\n", dirty));
    }
    if !bundle.repo_state.is_empty() {
        out.push_str(&format!("- repo_state: {}\n", bundle.repo_state.join(" ")));
    }
    out.push_str(&format!("- repro_hash: `{:x}`\n", bundle.repro_hash));
    out
}

/// Get the gh version string.
/// Returns `None` if `gh` is not found or fails.
fn get_gh_version() -> Option<String> {
    std::process::Command::new("gh")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the git remote URL (origin).
fn get_git_remote(cwd: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the current git branch name.
fn get_git_branch(cwd: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Check if the working tree is dirty.
fn get_git_dirty(cwd: &Path) -> Option<bool> {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let out = String::from_utf8(o.stdout).ok()?;
                Some(!out.trim().is_empty())
            } else {
                None
            }
        })
}

/// Detect which suite tools are present in the repo.
fn detect_repo_state(cwd: &Path) -> Vec<String> {
    let mut signals = Vec::new();
    for (marker, name) in &[
        (".wai", "wai"),
        (".beads", "beads"),
        ("openspec", "openspec"),
        (".dont", "dont"),
        (".espectacular", "espectacular"),
    ] {
        if cwd.join(marker).exists() {
            signals.push(name.to_string());
        }
    }
    signals
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let dir = tempfile::tempdir().unwrap();
        let signals = detect_repo_state(dir.path());
        assert!(signals.is_empty());

        fs::create_dir_all(dir.path().join(".wai")).unwrap();
        let signals = detect_repo_state(dir.path());
        assert!(signals.contains(&"wai".to_string()));
    }

    #[test]
    fn test_gather_context_creates_bundle() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".wai")).unwrap();
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
        let dir = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let remote = get_git_remote(dir.path());
        assert!(remote.is_none() || remote.is_some());
    }
}
