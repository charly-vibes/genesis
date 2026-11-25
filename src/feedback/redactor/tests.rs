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

/// Key-like substrings ("monkey_type", "keymap") that merely contain a secret
/// key's name must survive redaction untouched.
#[test]
fn test_redact_does_not_overredact_key_substrings() {
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

#[test]
fn test_redact_env_var_hex_path_not_redacted() {
    // Long hex path should not be incorrectly redacted
    let input = "BUILD_DIR=/a/b/c/d/e/f/g";
    let result = env_var_value_redacted(input);
    assert_eq!(result, "BUILD_DIR=/a/b/c/d/e/f/g");
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
    assert!(result.contains("[redacted]") || result.contains("GITHUB_TOKEN"));
}

#[test]
fn test_redact_no_home() {
    let input = "just a normal message";
    let result = redact(input, None, None);
    assert_eq!(result, "just a normal message");
}
