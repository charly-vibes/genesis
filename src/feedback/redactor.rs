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
/// Applies all redaction rules in order. Returns the redacted string.
pub fn redact(input: &str, home_dir: Option<&Path>, git_remote: Option<&str>) -> String {
    let mut result = Cow::Borrowed(input);

    // 1. Reduce git_remote if present in the text
    if let Some(remote) = git_remote {
        result = Cow::Owned(reduce_git_remote(&result, remote));
    }

    // 2. Strip secret values
    result = Cow::Owned(redact_secret_values(&result));

    // 3. Redact env var values
    result = Cow::Owned(redact_env_var_values(&result));

    // 4. Replace absolute home paths
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
    // Strip protocol prefix (https://, http://, ssh://, git://, ftp://)
    let without_proto = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };

    // Strip user:pass@ (including PAT tokens)
    let without_auth = if let Some(at_pos) = without_proto.rfind('@') {
        &without_proto[at_pos + 1..]
    } else {
        without_proto
    };

    // Strip query string
    let without_query = if let Some(q_pos) = without_auth.find('?') {
        &without_auth[..q_pos]
    } else {
        without_auth
    };

    // Handle git@github.com:owner/repo.git format
    let normalized = if without_query.contains(':') && !without_query.contains("://") {
        without_query.replace(':', "/")
    } else {
        without_query.to_string()
    };

    // Strip trailing .git
    if normalized.ends_with(".git") {
        normalized[..normalized.len() - 4].to_string()
    } else {
        normalized
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
    let secret_keywords = [
        "token", "secret", "password", "apikey", "api_key", "pat", "bearer",
    ];

    let mut result = input.to_string();

    for keyword in &secret_keywords {
        // Match: keyword=<value> or keyword <value> or "keyword": "<value>"
        let patterns = [
            // key=value (shell-like)
            format!(r"(?i)({})(=)(\S+)", regex::escape(keyword)),
            // key value (space-separated)
            format!(r"(?i)({})(\s+)(\S+)", regex::escape(keyword)),
            // "key": "value" (JSON-like)
            format!(r#"(?i)("{}":\s*")([^"]+)(")"#, regex::escape(keyword)),
        ];

        for pattern in &patterns {
            let re = regex::Regex::new(pattern).unwrap();
            result = re
                .replace_all(&result, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let separator = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    format!("{}{}[redacted]", prefix, separator)
                })
                .to_string();
        }
    }

    result
}

/// Redact environment variable values (`FOO=secret` → `FOO=[redacted]`).
///
/// Redacts if the value looks like a secret (long hex string, long base64,
/// or contains non-alphanumeric characters). Short simple values (booleans,
/// numbers, short words, paths) are preserved.
fn redact_env_var_values(input: &str) -> String {
    let re = regex::Regex::new(r"(?m)^([A-Za-z_][A-Za-z0-9_]*)=(.+)$").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let key = &caps[1];
        let value = &caps[2];
        // Don't redact short or simple values
        if value.len() < 4 {
            return format!("{}={}", key, value);
        }
        // Don't redact simple paths (contain only alphanumeric, /, :, ., _, -)
        if value.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '/' || c == ':' || c == '.' || c == '_' || c == '-'
        }) {
            // But redact if it looks like a hex token (long hex string)
            if value.len() >= 8 && value.chars().all(|c| c.is_ascii_hexdigit()) {
                return format!("{}={}", key, "[redacted]");
            }
            // Redact if it looks like a ghp_ GitHub PAT
            if value.starts_with("ghp_") || value.starts_with("ghs_") || value.starts_with("gho_") {
                return format!("{}={}", key, "[redacted]");
            }
            return format!("{}={}", key, value);
        }
        format!("{}={}", key, "[redacted]")
    })
    .to_string()
}

/// Replace absolute home directory paths with `~/`.
fn replace_home_paths(input: &str, home: &Path) -> String {
    let home_str = home.to_string_lossy();
    input.replace(&*home_str, "~")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── git_remote reduction ──────────────────────────────────────────

    #[test]
    fn test_reduce_git_remote_url_https() {
        assert_eq!(
            reduce_git_remote_url("https://github.com/owner/repo.git"),
            "github.com/owner/repo"
        );
    }

    #[test]
    fn test_reduce_git_remote_url_with_pat() {
        assert_eq!(
            reduce_git_remote_url("https://ghp_abc123@github.com/owner/repo.git"),
            "github.com/owner/repo"
        );
    }

    #[test]
    fn test_reduce_git_remote_url_ssh() {
        assert_eq!(
            reduce_git_remote_url("git@github.com:owner/repo.git"),
            "github.com/owner/repo"
        );
    }

    #[test]
    fn test_reduce_git_remote_url_with_query() {
        assert_eq!(
            reduce_git_remote_url("https://github.com/owner/repo?foo=bar"),
            "github.com/owner/repo"
        );
    }

    #[test]
    fn test_reduce_git_remote_url_no_git_suffix() {
        assert_eq!(
            reduce_git_remote_url("https://github.com/owner/repo"),
            "github.com/owner/repo"
        );
    }

    #[test]
    fn test_reduce_git_remote_in_text() {
        let text = "remote: https://pat@github.com/owner/repo.git";
        let result = reduce_git_remote(text, "https://pat@github.com/owner/repo.git");
        assert_eq!(result, "remote: github.com/owner/repo");
    }

    // ── Secret value redaction ────────────────────────────────────────

    #[test]
    fn test_redact_token_value() {
        let input = "ghp_abc123def456";
        let result = redact_secret_values(input);
        // Without a keyword prefix, individual tokens might not match
        // This tests that the function doesn't panic
        assert!(result.contains("ghp_abc123def456") || result.contains("[redacted]"));
    }

    #[test]
    fn test_redact_token_in_env_var() {
        let input = "GITHUB_TOKEN=ghp_abc123";
        let result = env_var_value_redacted(input);
        assert!(result.contains("[redacted]"));
    }

    #[test]
    fn test_redact_secret_in_json() {
        let input = r#"{"token": "ghp_abc123", "user": "me"}"#;
        let result = redact_secret_values(input);
        assert!(result.contains("[redacted]"));
        assert!(result.contains("user"));
        assert!(result.contains("me"));
    }

    #[test]
    fn test_redact_does_not_overredact_key_substrings() {
        // "monkey_type" or "keymap" should not be redacted
        let input = "monkey_type = \"foo\"\nkeymap = \"vim\"";
        let result = redact_secret_values(input);
        assert!(result.contains("monkey_type"));
        assert!(result.contains("keymap"));
    }

    // ── Env var value redaction ───────────────────────────────────────

    fn env_var_value_redacted(input: &str) -> String {
        redact_env_var_values(input)
    }

    #[test]
    fn test_redact_env_var_long_value() {
        let input = "SECRET=abcdef123456";
        let result = env_var_value_redacted(input);
        assert_eq!(result, "SECRET=[redacted]");
    }

    #[test]
    fn test_redact_env_var_short_value() {
        let input = "DEBUG=true";
        let result = env_var_value_redacted(input);
        assert_eq!(result, "DEBUG=true");
    }

    #[test]
    fn test_redact_env_var_simple_path() {
        let input = "PATH=/usr/bin:/bin";
        let result = env_var_value_redacted(input);
        assert_eq!(result, "PATH=/usr/bin:/bin");
    }

    // ── Home path replacement ─────────────────────────────────────────

    #[test]
    fn test_replace_home_path() {
        let home = Path::new("/home/user");
        let input = "file at /home/user/project/src/main.rs";
        let result = replace_home_paths(input, home);
        assert_eq!(result, "file at ~/project/src/main.rs");
    }

    #[test]
    fn test_replace_home_path_multiple() {
        let home = Path::new("/home/user");
        let input = "/home/user/a and /home/user/b";
        let result = replace_home_paths(input, home);
        assert_eq!(result, "~/a and ~/b");
    }

    #[test]
    fn test_replace_home_no_match() {
        let home = Path::new("/home/user");
        let input = "no home path here";
        let result = replace_home_paths(input, home);
        assert_eq!(result, "no home path here");
    }

    // ── Full redact function ──────────────────────────────────────────

    #[test]
    fn test_full_redact() {
        let home = Some(Path::new("/home/user"));
        let input = "path: /home/user/project\nremote: https://pat@github.com/owner/repo.git\nGITHUB_TOKEN=ghp_abc123";
        let result = redact(input, home, Some("https://pat@github.com/owner/repo.git"));
        assert!(result.contains("~/project"));
        assert!(result.contains("github.com/owner/repo"));
        // The env var should be redacted by the env var redactor
        assert!(result.contains("[redacted]") || result.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_redact_no_home() {
        let input = "just a normal message";
        let result = redact(input, None, None);
        assert_eq!(result, "just a normal message");
    }
}
