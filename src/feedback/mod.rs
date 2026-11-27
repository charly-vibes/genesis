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
//!     title: None,
//!     dry_run: true,
//!     from_last_error: true,
//! };
//!
//! genesis::feedback::handle_feedback(
//!     &args,
//!     "my-tool",
//!     "0.1.0",
//!     "owner/my-tool",
//!     &std::env::current_dir().unwrap(),
//! ).unwrap();
//! ```

pub mod context;
pub mod gh;
pub mod redactor;
pub mod scratch;

use content::{IssueContent, apply_title_override, issue_title_and_body};
mod content;

use crate::suggestions::{CommandRegistry, SuggestionEngine};
use std::path::Path;

/// Arguments for the unified feedback handler.
///
/// Mirrors the `clap` subcommand struct that each tool defines.
#[derive(Debug, Clone)]
pub struct FeedbackArgs {
    /// Kind of feedback: "bug", "feature", "question", or "chore".
    pub kind: String,
    /// User-supplied issue title. When set, overrides the derived title
    /// ("feedback report" / "auto-reported error: …" / first stdin line).
    pub title: Option<String>,
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
            title: None,
            dry_run,
            from_last_error,
        }
    }

    /// Set a user-supplied issue title (overrides the derived title).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Valid issue kinds. `aix-gap` reports AIX-artifact failures
/// (stale generated docs, drifted managed blocks) per add-artifact-provenance §5.
const VALID_KINDS: &[&str] = &["bug", "feature", "question", "chore", "aix-gap"];

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
/// # Errors
///
/// Returns an error if `kind` is invalid (validated with typo suggestions before this
/// is ever reached, but a guard is in place anyway).
pub fn handle_feedback(
    args: &FeedbackArgs,
    tool_name: &str,
    tool_version: &str,
    repo: &str,
    project_root: &Path,
) -> Result<gh::GhResult, String> {
    validate_kind(&args.kind)?;

    let content = assembled_content(args, tool_name, tool_version, project_root)?;
    let redacted = redact_body(&content.body_parts, repo);

    file_issue(args, repo, content.title, redacted)
}

/// Assemble the issue content: title/body from the args' content source,
/// the title override applied, and the context bundle appended to the
/// body parts.
fn assembled_content(
    args: &FeedbackArgs,
    tool_name: &str,
    tool_version: &str,
    project_root: &Path,
) -> Result<IssueContent, String> {
    let mut content = issue_title_and_body(args, tool_name)?;
    content.title = apply_title_override(&args.kind, &content.title, args.title.as_deref());
    content.body_parts.push(context_bundle_section(
        tool_name,
        tool_version,
        project_root,
        content.command.take(),
        content.exit_code,
        content.suggestion_footer.take(),
    ));
    Ok(content)
}

/// Validate the issue kind, with a typo suggestion when close to a valid one.
fn validate_kind(kind: &str) -> Result<(), String> {
    if VALID_KINDS.contains(&kind) {
        return Ok(());
    }
    let mut reg = CommandRegistry::new();
    reg.register("kind", VALID_KINDS.iter().map(|k| k.to_string()).collect());
    let engine = SuggestionEngine::new();
    if let Some(suggestion) = engine.suggest_typo(kind, &reg) {
        return Err(format!(
            "unknown kind: '{}'. {}",
            kind,
            suggestion.message()
        ));
    }
    Err(format!(
        "unknown kind: '{}'. Valid kinds: {}",
        kind,
        VALID_KINDS.join(", ")
    ))
}

/// Gather the context bundle and format it as the trailing body section.
/// `command`, `exit_code`, and `suggestion_footer` come from the scratch
/// record when one backed the body (wai reference wiring), so the
/// `## Environment` section matches the reported failure.
fn context_bundle_section(
    tool_name: &str,
    tool_version: &str,
    project_root: &Path,
    command: Option<String>,
    exit_code: Option<i32>,
    suggestion_footer: Option<String>,
) -> String {
    let bundle = context::gather_context(
        tool_name,
        tool_version,
        command,
        exit_code,
        suggestion_footer,
        project_root,
    );
    context::format_context_bundle(&bundle)
}

/// Join body sections, then redact home paths and the target repo.
fn redact_body(body_parts: &[String], repo: &str) -> String {
    let body = body_parts.join("\n\n");
    let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
    redactor::redact(&body, home.as_deref(), Some(repo))
}

/// Labels for a given issue kind.
fn labels_for_kind(kind: &str) -> Vec<String> {
    let mut labels = vec!["agent-reported".to_string()];
    match kind {
        "bug" => {
            labels.push("bug".to_string());
            labels.push("has-repro".to_string());
        }
        "feature" => labels.push("enhancement".to_string()),
        "question" => labels.push("question".to_string()),
        "chore" => labels.push("chore".to_string()),
        _ => {}
    }
    labels
}

/// Print a dry-run preview, or create the issue via `gh`.
fn file_issue(
    args: &FeedbackArgs,
    repo: &str,
    title: String,
    body: String,
) -> Result<gh::GhResult, String> {
    let labels = labels_for_kind(&args.kind);
    if args.dry_run {
        return dry_run_result(repo, &title, &labels, &body);
    }
    let opts = gh::CreateIssueOptions {
        repo: repo.to_string(),
        title,
        body,
        labels,
        dry_run: false,
    };
    gh::create_issue(&opts)
}

/// Print the dry-run preview and return the new-issue fallback URL.
fn dry_run_result(
    repo: &str,
    title: &str,
    labels: &[String],
    body: &str,
) -> Result<gh::GhResult, String> {
    eprintln!("{body}");
    eprintln!();
    eprintln!(
        "Would file: gh issue create --repo {repo} --title \"{title}\" --label {}",
        labels.join(", ")
    );
    Ok(gh::GhResult::FallbackUrl(format!(
        "https://github.com/{repo}/issues/new"
    )))
}

#[cfg(test)]
mod tests;

pub use scenario::{ConversionError, from_last_error};

mod scenario;
