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
    let mut bundle = core_bundle(
        tool_name,
        tool_version,
        command,
        exit_code,
        suggestion_footer,
    );
    bundle.apply_env();
    bundle.apply_git(cwd);
    bundle
}

/// The caller-supplied fields; [`ContextBundle::apply_env`] and
/// [`ContextBundle::apply_git`] fill in the probed remainder.
fn core_bundle(
    tool_name: &str,
    tool_version: &str,
    command: Option<String>,
    exit_code: Option<i32>,
    suggestion_footer: Option<String>,
) -> ContextBundle {
    let repro_hash = repro_hash(tool_name, tool_version, &command, exit_code);
    ContextBundle {
        tool_name: tool_name.to_string(),
        tool_version: tool_version.to_string(),
        command,
        exit_code,
        suggestion_footer,
        repro_hash,
        ..ContextBundle::default()
    }
}

impl ContextBundle {
    /// Fill the environment-derived fields (platform, shell, gh version).
    fn apply_env(&mut self) {
        self.os_arch = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
        self.shell = std::env::var("SHELL").ok();
        self.gh_version = get_gh_version();
    }

    /// Fill the git-derived fields probed at `cwd`.
    fn apply_git(&mut self, cwd: &Path) {
        let git = probe_git(cwd);
        self.git_remote = git.remote;
        self.git_branch = git.branch;
        self.git_dirty = git.dirty;
        self.repo_state = git.state;
    }
}

/// Git context probed from `cwd` in one pass.
struct GitProbe {
    remote: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    state: Vec<String>,
}

/// Probe the git repository state at `cwd`.
fn probe_git(cwd: &Path) -> GitProbe {
    GitProbe {
        remote: get_git_remote(cwd),
        branch: get_git_branch(cwd),
        dirty: get_git_dirty(cwd),
        state: detect_repo_state(cwd),
    }
}

/// Build a repro hash from tool + command + exit_code.
fn repro_hash(
    tool_name: &str,
    tool_version: &str,
    command: &Option<String>,
    exit_code: Option<i32>,
) -> u64 {
    let repro_input = format!(
        "{}:{}:{}:{:?}",
        tool_name,
        tool_version,
        command.as_deref().unwrap_or(""),
        exit_code
    );
    compute_hash(&repro_input)
}

/// Compute a simple hash for dedup purposes.
fn compute_hash(input: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Serialize the context bundle to a markdown section.
pub fn format_context_bundle(bundle: &ContextBundle) -> String {
    let mut out = String::from("## Environment\n\n");
    out.push_str(&format!(
        "- tool: {} {}\n",
        bundle.tool_name, bundle.tool_version
    ));
    out.push_str(&optional_fields(bundle));
    out.push_str(&format!("- os/arch: {}\n", bundle.os_arch));
    if !bundle.repo_state.is_empty() {
        out.push_str(&format!("- repo_state: {}\n", bundle.repo_state.join(" ")));
    }
    out.push_str(&format!("- repro_hash: `{:x}`\n", bundle.repro_hash));
    out
}

/// One `- key: value` line when `value` is present.
fn opt_line<V: std::fmt::Display>(key: &str, value: Option<V>) -> Option<String> {
    value.map(|v| format!("- {key}: {v}\n"))
}

/// The optional environment fields, one `- key: value` line each.
fn optional_fields(b: &ContextBundle) -> String {
    [
        opt_line("command", b.command.as_deref()),
        opt_line("exit_code", b.exit_code),
        opt_line("suggestion", b.suggestion_footer.as_deref()),
        opt_line("shell", b.shell.as_deref()),
        opt_line("gh_version", b.gh_version.as_deref()),
        opt_line("git_remote", b.git_remote.as_deref()),
        opt_line("git_branch", b.git_branch.as_deref()),
        opt_line("git_dirty", b.git_dirty),
    ]
    .into_iter()
    .flatten()
    .collect::<String>()
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

/// Spawn git anchored at `cwd`, ignoring hook-context injection.
///
/// `git commit` exports `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` to
/// hook processes, and this code runs from inside hooks (feedback comes
/// from agent sessions); the context must describe the workspace at
/// `cwd`, never the ambient `GIT_DIR`'s repository.
fn git_cmd(cwd: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
        cmd.env_remove(var);
    }
    cmd.current_dir(cwd);
    cmd
}

/// Get the git remote URL (origin).
fn get_git_remote(cwd: &Path) -> Option<String> {
    git_cmd(cwd)
        .args(["config", "--get", "remote.origin.url"])
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
/// `git branch --show-current` (not `rev-parse --abbrev-ref HEAD`) so a
/// fresh repo on an unborn branch still reports its branch name (git
/// ≥ 2.22; older git fails the command and degrades to `None`). Empty
/// output (detached HEAD) also degrades to `None` (genesis-w6h).
fn get_git_branch(cwd: &Path) -> Option<String> {
    git_cmd(cwd)
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
}

/// Check if the working tree is dirty.
fn get_git_dirty(cwd: &Path) -> Option<bool> {
    git_cmd(cwd)
        .args(["status", "--porcelain"])
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
    [
        (".wai", "wai"),
        (".beads", "beads"),
        ("openspec", "openspec"),
        (".dont", "dont"),
        (".espectacular", "espectacular"),
    ]
    .into_iter()
    .filter(|(marker, _)| cwd.join(marker).exists())
    .map(|(_, name)| name.to_string())
    .collect()
}

#[cfg(test)]
#[path = "tests.rs"]
#[cfg(test)]
mod tests;
