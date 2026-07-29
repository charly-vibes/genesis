//! Agent issue reporting (feedback).
//!
//! Provides a unified [`handle_feedback`] function that any tool can call
//! from its `feedback` subcommand. Wraps the individual modules (scratch,
//! context, redactor, gh) into one entry point.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use genesis::feedback::FeedbackArgs;
//!
//! let args = FeedbackArgs {
//!     kind: "bug".to_string(),
//!     dry_run: true,
//!     from_last_error: true,
//! };
//!
//! genesis::feedback::handle_feedback(
//!     &args,
//!     "my-tool",
//!     "0.1.0",
//!     "owner/my-tool",
//! ).unwrap();
//! ```

pub mod context;
pub mod gh;
pub mod redactor;
pub mod scratch;

use crate::suggestions::{CommandRegistry, SuggestionEngine};
use std::path::Path;

/// Arguments for the unified feedback handler.
///
/// Mirrors the `clap` subcommand struct that each tool defines.
#[derive(Debug, Clone)]
pub struct FeedbackArgs {
    /// Kind of feedback: "bug", "feature", "question", or "chore".
    pub kind: String,
    /// If true, print the issue body and gh command without submitting.
    pub dry_run: bool,
    /// If true, read the last error from scratch to auto-populate the body.
    pub from_last_error: bool,
}

impl FeedbackArgs {
    /// Create a new FeedbackArgs with the given values.
    pub fn new(kind: impl Into<String>, dry_run: bool, from_last_error: bool) -> Self {
        Self {
            kind: kind.into(),
            dry_run,
            from_last_error,
        }
    }
}

/// Valid issue kinds.
const VALID_KINDS: &[&str] = &["bug", "feature", "question", "chore"];

/// Handle a feedback request — validate kind, build body, redact, and file.
///
/// # Arguments
///
/// * `args` — The parsed CLI arguments for the feedback subcommand.
/// * `tool_name` — The tool's name (e.g., `"testaruda"`).
/// * `tool_version` — The tool's version string.
/// * `repo` — The GitHub repository in `"owner/repo"` format.
/// * `project_root` — The repo root directory (for context gathering and git detection).
///
/// # Returns
///
/// The [`gh::GhResult`] on success, or an error message on failure.
///
/// # Panics
///
/// Panics if `kind` is invalid (validated with typo suggestions before this
/// is ever reached, but a guard is in place anyway).
pub fn handle_feedback(
    args: &FeedbackArgs,
    tool_name: &str,
    tool_version: &str,
    repo: &str,
    project_root: &Path,
) -> Result<gh::GhResult, String> {
    // ── Validate kind with suggestions ─────────────────────────────
    if !VALID_KINDS.contains(&args.kind.as_str()) {
        let mut reg = CommandRegistry::new();
        reg.register("kind", VALID_KINDS.iter().map(|k| k.to_string()).collect());
        let engine = SuggestionEngine::new();
        if let Some(suggestion) = engine.suggest_typo(&args.kind, &reg) {
            return Err(format!(
                "unknown kind: '{}'. {}",
                args.kind,
                suggestion.message()
            ));
        }
        return Err(format!(
            "unknown kind: '{}'. Valid kinds: {}",
            args.kind,
            VALID_KINDS.join(", ")
        ));
    }

    // ── Build issue body ───────────────────────────────────────────
    let mut body_parts: Vec<String> = Vec::new();
    let mut title = format!("[{}] ", args.kind);

    if args.from_last_error {
        if let Some(record) = scratch::read_last_error(tool_name) {
            let cmd_str = record.argv.join(" ");
            title.push_str(&format!("auto-reported error: {}", cmd_str));
            body_parts.push("## Error\n\n".to_string());
            body_parts.push(format!("**Command:** `{}`\n\n", cmd_str));
            body_parts.push(format!("**Exit code:** {}\n\n", record.exit));
            if let Some(ref footer) = record.footer {
                body_parts.push(format!("**Suggestion:** {}\n\n", footer));
            }
        } else {
            return Err(format!(
                "No recent error recorded for '{}'. Run a command that produces an error first.",
                tool_name
            ));
        }
    } else {
        return Err(
            "No issue content specified. Use --from-last-error or pipe issue details into stdin."
                .to_string(),
        );
    }

    // ── Append context bundle ──────────────────────────────────────
    let bundle = context::gather_context(tool_name, tool_version, None, None, None, project_root);
    body_parts.push(context::format_context_bundle(&bundle));

    // ── Redact sensitive info ──────────────────────────────────────
    let body = body_parts.join("\n\n");
    let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
    let redacted = redactor::redact(&body, home.as_deref(), Some(repo));

    // ── Determine labels ───────────────────────────────────────────
    let labels = match args.kind.as_str() {
        "bug" => vec![
            "agent-reported".to_string(),
            "bug".to_string(),
            "has-repro".to_string(),
        ],
        "feature" => vec!["agent-reported".to_string(), "enhancement".to_string()],
        "question" => vec!["agent-reported".to_string(), "question".to_string()],
        "chore" => vec!["agent-reported".to_string(), "chore".to_string()],
        _ => vec!["agent-reported".to_string()],
    };

    if args.dry_run {
        eprintln!("{}", redacted);
        eprintln!();
        eprintln!(
            "Would file: gh issue create --repo {} --title \"{}\" --label {}",
            repo,
            title,
            labels.join(", ")
        );
        return Ok(gh::GhResult::FallbackUrl(format!(
            "https://github.com/{}/issues/new",
            repo
        )));
    }

    // ── Create issue via gh ────────────────────────────────────────
    let opts = gh::CreateIssueOptions {
        repo: repo.to_string(),
        title,
        body: redacted,
        labels,
        dry_run: false,
    };

    gh::create_issue(&opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scratch::ErrorRecord;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write_scratch(tool: &str, exit_code: i32) {
        let record = ErrorRecord {
            ts: "2026-07-29T12:00:00Z".into(),
            argv: vec![tool.to_string(), "check".into()],
            exit: exit_code,
            footer: Some("run doctor".into()),
            kind: "error".into(),
        };
        scratch::write_scratch_best_effort(tool, &record);
    }

    #[test]
    fn test_handle_feedback_dry_run() {
        let dir = tmp();
        write_scratch("test-tool", 1);

        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
        assert!(result.is_ok(), "dry run should succeed: {:?}", result);
    }

    #[test]
    fn test_handle_feedback_from_last_error_no_scratch() {
        let dir = tmp();
        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, "no-such-tool", "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should fail when no scratch exists");
        assert!(
            result.unwrap_err().contains("No recent error"),
            "should mention missing scratch"
        );
    }

    #[test]
    fn test_handle_feedback_no_from_last_error() {
        let dir = tmp();
        let args = FeedbackArgs::new("bug", true, false);
        let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should fail without --from-last-error");
        assert!(
            result.unwrap_err().contains("--from-last-error"),
            "should suggest --from-last-error"
        );
    }

    #[test]
    fn test_handle_feedback_invalid_kind() {
        let dir = tmp();
        let args = FeedbackArgs::new("invalid-kind", true, true);
        let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should reject invalid kind");
        let err = result.unwrap_err();
        assert!(err.contains("unknown kind"), "should say 'unknown kind'");
    }

    #[test]
    fn test_handle_feedback_typo_suggestion_for_kind() {
        let dir = tmp();
        let args = FeedbackArgs::new("featuer", true, true);
        let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should reject typo kind");
        let err = result.unwrap_err();
        // Should contain a 'Did you mean' suggestion
        assert!(
            err.contains("feature") || err.contains("Did you mean"),
            "typo should suggest correct kind: {}",
            err
        );
    }

    #[test]
    fn test_valid_kinds_list_matches_validation() {
        for kind in VALID_KINDS {
            let args = FeedbackArgs::new(kind.to_string(), true, true);
            let dir = tmp();
            write_scratch("test-tool", 1);
            let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
            assert!(
                result.is_ok(),
                "kind '{}' should be valid: {:?}",
                kind,
                result
            );
        }
    }

    #[test]
    fn test_dry_run_prints_to_stderr() {
        // Verify dry_run returns FallbackUrl (not actually creating issue)
        let dir = tmp();
        write_scratch("test-tool", 1);
        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, "test-tool", "0.1.0", "owner/repo", dir.path());
        match result {
            Ok(gh::GhResult::FallbackUrl(url)) => {
                assert!(url.contains("github.com"), "URL should contain github.com");
            }
            Ok(other) => panic!("expected FallbackUrl, got {:?}", other),
            Err(e) => panic!("should not error: {}", e),
        }
    }
}
