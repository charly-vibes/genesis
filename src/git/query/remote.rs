//! Remote URL queries and `remote_key` canonicalization — pure string
//! reduction over `git remote get-url` results.

use std::path::Path;

use super::{EnvPolicy, GitError, run};

/// The URL of a named remote (conventionally `origin`), or `None` when
/// the remote does not exist (`git remote get-url` exits 2).
pub fn remote_url(root: &Path, name: &str) -> Result<Option<String>, GitError> {
    remote_url_with_policy(root, name, EnvPolicy::Inherit)
}

/// [`remote_url`](super::remote_url) with an explicit environment policy
/// (policy-aware variant; see [`super::run`] for what the policy controls).
pub fn remote_url_with_policy(
    root: &Path,
    name: &str,
    policy: EnvPolicy,
) -> Result<Option<String>, GitError> {
    match run(root, policy, &["remote", "get-url", name]) {
        Ok(out) => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok((!url.is_empty()).then_some(url))
        }
        // exit 2: "No such remote" — an absent remote is an answer,
        // not a failure.
        Err(GitError::Git { code: 2, .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// Reduce a git remote URL to `host/path`, dropping credentials, protocol,
/// and query. Pure function — no git spawn.
///
/// Examples:
/// - `https://pat@github.com/owner/repo.git` → `github.com/owner/repo`
/// - `git@github.com:owner/repo.git` → `github.com/owner/repo`
/// - `https://github.com/owner/repo` → `github.com/owner/repo`
///
/// Relocated from `feedback::redactor` (which re-exports it under its
/// former name) so remote-URL canonicalization has one home.
pub fn remote_key(url: &str) -> String {
    let without_auth = strip_protocol_and_auth(url);
    let without_query = strip_query_string(without_auth);
    let normalized = normalize_scp_syntax(without_query);
    strip_git_suffix(&normalized)
}

/// Strip protocol prefix (https://, http://, ssh://, git://, ftp://) and
/// user:pass@ credentials (including PAT tokens).
fn strip_protocol_and_auth(url: &str) -> &str {
    let without_proto = match url.find("://") {
        Some(pos) => &url[pos + 3..],
        None => url,
    };
    match without_proto.rfind('@') {
        Some(at_pos) => &without_proto[at_pos + 1..],
        None => without_proto,
    }
}

/// Strip a trailing query string.
fn strip_query_string(s: &str) -> &str {
    match s.find('?') {
        Some(q_pos) => &s[..q_pos],
        None => s,
    }
}

/// Rewrite `git@github.com:owner/repo` scp-style syntax to `owner/repo`.
fn normalize_scp_syntax(s: &str) -> String {
    if s.contains(':') && !s.contains("://") {
        s.replace(':', "/")
    } else {
        s.to_string()
    }
}

/// Strip a trailing `.git` suffix.
fn strip_git_suffix(s: &str) -> String {
    match s.strip_suffix(".git") {
        Some(stripped) => stripped.to_string(),
        None => s.to_string(),
    }
}
