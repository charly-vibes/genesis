//! Privacy redactor for issue bodies (§5 of agent-issue-reporting.md).
//!
//! # Redaction rules
//!
//! 1. **Reduce `git_remote`** to `host/path` (drop `user:pass@` and query).
//! 2. **Strip secret *values*** matching common patterns
//!    (`(?i)(token|secret|password|apikey|pat|bearer)`).
//! 3. **Redact env var values** (`FOO=secret` → `FOO=[redacted]`).
//! 4. **Replace absolute home paths** with `~/`.
//! 5. **Match values, not key substrings** — avoid over-redacting
//!    terms like `monkey_type` or `keymap`.
//! 6. **Mark elided ranges** with `[redacted]`.

use std::borrow::Cow;
use std::path::Path;

/// Redact sensitive information from a string.
///
/// Applies all redaction rules in order — git remote reduction, secret
/// value stripping, env var value redaction, home path replacement —
/// and returns the redacted string.
pub fn redact(input: &str, home_dir: Option<&Path>, git_remote: Option<&str>) -> String {
    let mut result = Cow::Borrowed(input);
    if let Some(remote) = git_remote {
        result = Cow::Owned(reduce_git_remote(&result, remote));
    }
    result = Cow::Owned(redact_secret_values(&result));
    result = Cow::Owned(redact_env_var_values(&result));
    if let Some(home) = home_dir {
        result = Cow::Owned(replace_home_paths(&result, home));
    }
    result.into_owned()
}

/// Reduce a git remote URL to `host/path`, dropping credentials, protocol, and query.
///
/// Examples:
/// - `https://pat@github.com/owner/repo.git` → `github.com/owner/repo`
/// - `git@github.com:owner/repo.git` → `github.com/owner/repo`
/// - `https://github.com/owner/repo` → `github.com/owner/repo`
pub fn reduce_git_remote_url(url: &str) -> String {
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

/// Replace occurrences of a git remote URL in text with its reduced form.
fn reduce_git_remote(text: &str, remote: &str) -> String {
    let reduced = reduce_git_remote_url(remote);
    text.replace(remote, &reduced)
}

/// Redact values that look like secrets.
///
/// Matches common secret patterns:
/// - `token=abc123` → `token=[redacted]`
/// - `--token abc123` → `--token [redacted]`
/// - `"secret": "abc123"` → `"secret": "[redacted]"`
fn redact_secret_values(input: &str) -> String {
    const SECRET_KEYWORDS: [&str; 7] = [
        "token", "secret", "password", "apikey", "api_key", "pat", "bearer",
    ];

    let mut result = input.to_string();
    for keyword in SECRET_KEYWORDS {
        for pattern in secret_patterns(keyword) {
            result = apply_redaction(&result, &pattern);
        }
    }
    result
}

/// Patterns matching a secret next to `keyword`:
/// `keyword=<value>`, `keyword <value>`, and `"keyword": "<value>"`.
fn secret_patterns(keyword: &str) -> [String; 3] {
    let kw = regex::escape(keyword);
    [
        // key=value (shell-like)
        format!(r"(?i)({})(=)(\S+)", kw),
        // key value (space-separated)
        format!(r"(?i)({})(\s+)(\S+)", kw),
        // "key": "value" (JSON-like)
        format!(r#"(?i)("{}":\s*")([^"]+)(")"#, kw),
    ]
}

/// Replace every match of `pattern` with its captured key prefix plus
/// `[redacted]`.
fn apply_redaction(input: &str, pattern: &str) -> String {
    let re = regex::Regex::new(pattern).expect("static redaction pattern compiles");
    re.replace_all(input, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let separator = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        format!("{prefix}{separator}[redacted]")
    })
    .to_string()
}

/// Whether an env var value must be redacted or kept verbatim.
enum EnvValueDisposition {
    Keep,
    Redact,
}

/// Classify an env var value. Short simple values (booleans, numbers,
/// short words, paths) are kept; long hex strings, GitHub PATs, and
/// values with non-alphanumeric characters are redacted.
fn env_value_disposition(value: &str) -> EnvValueDisposition {
    if value.len() < 4 {
        return EnvValueDisposition::Keep;
    }
    if is_simple_path(value) && !(is_long_hex(value) || is_github_pat(value)) {
        return EnvValueDisposition::Keep;
    }
    EnvValueDisposition::Redact
}

/// Only alphanumeric plus path punctuation (`/`, `:`, `.`, `_`, `-`).
fn is_simple_path(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | ':' | '.' | '_' | '-'))
}

/// A hex token of at least eight characters.
fn is_long_hex(value: &str) -> bool {
    value.len() >= 8 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// A `ghp_`/`ghs_`/`gho_` GitHub PAT prefix.
fn is_github_pat(value: &str) -> bool {
    value.starts_with("ghp_") || value.starts_with("ghs_") || value.starts_with("gho_")
}

/// Redact environment variable values (`FOO=secret` → `FOO=[redacted]`).
///
/// Redacts if the value looks like a secret (long hex string, long base64,
/// or contains non-alphanumeric characters). Short simple values (booleans,
/// numbers, short words, paths) are preserved.
fn redact_env_var_values(input: &str) -> String {
    let re = regex::Regex::new(r"(?m)^([A-Za-z_][A-Za-z0-9_]*)=(.+)$").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let (key, value) = (&caps[1], &caps[2]);
        match env_value_disposition(value) {
            EnvValueDisposition::Keep => format!("{key}={value}"),
            EnvValueDisposition::Redact => format!("{key}=[redacted]"),
        }
    })
    .to_string()
}

/// Replace absolute home directory paths with `~/`.
fn replace_home_paths(input: &str, home: &Path) -> String {
    let home_str = home.to_string_lossy();
    input.replace(&*home_str, "~")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
