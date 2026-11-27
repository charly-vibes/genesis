//! Checkout-shape facts and branch queries: `git rev-parse --git-dir
//! --git-common-dir` and `git branch --show-current`.

use std::path::Path;

use super::super::runner::lines_of;
use super::{EnvPolicy, GitError, run};

/// Checkout facts from `git rev-parse --git-dir --git-common-dir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeFacts {
    /// The `.git` directory of the current checkout (relative when the
    /// checkout sits at the main working tree, absolute elsewhere).
    pub git_dir: String,
    /// The per-repository common dir: equals `git_dir` for a plain
    /// checkout, the main repository's `.git` for a linked worktree,
    /// and the bare directory itself in a bare repo.
    pub common_dir: String,
    /// `git_dir != common_dir` — a linked worktree checkout.
    pub is_linked_worktree: bool,
}

/// Detect the checkout shape at `root` (git-dir vs common-dir).
pub fn worktree_facts(root: &Path) -> Result<WorktreeFacts, GitError> {
    worktree_facts_with_policy(root, EnvPolicy::Inherit)
}

/// [`worktree_facts`](super::worktree_facts) with an explicit environment
/// policy (policy-aware variant; see [`super::run`] for what the policy
/// controls).
pub fn worktree_facts_with_policy(
    root: &Path,
    policy: EnvPolicy,
) -> Result<WorktreeFacts, GitError> {
    let out = run(
        root,
        policy,
        &["rev-parse", "--git-dir", "--git-common-dir"],
    )?;
    parse_git_dir_pair(&out.stdout)
}

/// Parse the two `rev-parse` lines (git-dir, common-dir); a missing
/// second line is a malformed environment, never silently treatable.
fn parse_git_dir_pair(stdout: &[u8]) -> Result<WorktreeFacts, GitError> {
    let mut lines = lines_of(stdout).into_iter();
    let (Some(git_dir), Some(common_dir)) = (lines.next(), lines.next()) else {
        return Err(GitError::Git {
            args: vec![
                "rev-parse".into(),
                "--git-dir".into(),
                "--git-common-dir".into(),
            ],
            code: 0,
            stderr: "unexpected output: expected two lines (git-dir, common-dir)".into(),
        });
    };
    Ok(WorktreeFacts {
        is_linked_worktree: git_dir != common_dir,
        git_dir,
        common_dir,
    })
}

/// The checked-out branch name at `root`, or `None` when detached or
/// unborn (`git branch --show-current` prints nothing for both).
pub fn current_branch(root: &Path) -> Result<Option<String>, GitError> {
    current_branch_with_policy(root, EnvPolicy::Inherit)
}

/// [`current_branch`](super::current_branch) with an explicit environment
/// policy (policy-aware variant; see [`super::run`] for what the policy
/// controls).
pub fn current_branch_with_policy(
    root: &Path,
    policy: EnvPolicy,
) -> Result<Option<String>, GitError> {
    let out = run(root, policy, &["branch", "--show-current"])?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((!branch.is_empty()).then_some(branch))
}
