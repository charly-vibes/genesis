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

/// Partition piped stdin into an issue title and a Description body.
///
/// - Single-line input (including a trailing newline, e.g. from `echo`)
///   keeps the generic `[{kind}] feedback report` title and lands verbatim
///   in the Description — backward compatible with single-line pipelines.
/// - Multi-line input promotes the first line to the title and uses the
///   remainder as the Description. The full input is read, so multi-
///   paragraph reports are never truncated.
fn stdin_title_and_body(kind: &str, input: &str) -> (String, String) {
    let trimmed = input.trim();
    if let Some((first, rest)) = trimmed.split_once('\n') {
        let first = first.trim();
        let rest = rest.trim();
        if !first.is_empty() {
            let title = format!("[{kind}] {first}");
            let body = if rest.is_empty() {
                String::new()
            } else {
                format!("## Description\n\n{rest}\n\n")
            };
            return (title, body);
        }
    }
    (
        format!("[{kind}] feedback report"),
        format!("## Description\n\n{trimmed}\n\n"),
    )
}

/// Apply a user-supplied title override, keeping the `[{kind}] ` prefix.
/// `None` keeps the derived title unchanged.
fn apply_title_override(kind: &str, derived: &str, title: Option<&str>) -> String {
    match title {
        Some(t) => format!("[{kind}] {t}"),
        None => derived.to_string(),
    }
}

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

    let (mut title, mut body_parts) = issue_title_and_body(args, tool_name)?;

    // User-supplied title wins over every derived title.
    title = apply_title_override(&args.kind, &title, args.title.as_deref());
    body_parts.push(context_bundle_section(
        tool_name,
        tool_version,
        project_root,
    ));
    let redacted = redact_body(&body_parts, repo);

    file_issue(args, repo, title, redacted)
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
fn context_bundle_section(tool_name: &str, tool_version: &str, project_root: &Path) -> String {
    let bundle = context::gather_context(tool_name, tool_version, None, None, None, project_root);
    context::format_context_bundle(&bundle)
}

/// Join body sections, then redact home paths and the target repo.
fn redact_body(body_parts: &[String], repo: &str) -> String {
    let body = body_parts.join("\n\n");
    let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
    redactor::redact(&body, home.as_deref(), Some(repo))
}

/// Derive the issue title and body sections from the content source:
/// the last scratch error (`--from-last-error`) or piped stdin.
fn issue_title_and_body(
    args: &FeedbackArgs,
    tool_name: &str,
) -> Result<(String, Vec<String>), String> {
    if args.from_last_error {
        title_and_body_from_error(&args.kind, tool_name)
    } else {
        title_and_body_from_stdin(&args.kind)
    }
}

/// Title/body from the last recorded error for `tool_name`.
fn title_and_body_from_error(kind: &str, tool_name: &str) -> Result<(String, Vec<String>), String> {
    let Some(record) = scratch::read_last_error(tool_name) else {
        return Err(format!(
            "No recent error recorded for '{}'. Run a command that produces an error first.",
            tool_name
        ));
    };
    let cmd_str = record.argv.join(" ");
    let title = format!("[{kind}] auto-reported error: {cmd_str}");
    let mut body_parts = vec!["## Error\n\n".to_string()];
    body_parts.push(format!("**Command:** `{cmd_str}`\n\n"));
    body_parts.push(format!("**Exit code:** {}\n\n", record.exit));
    if let Some(ref footer) = record.footer {
        body_parts.push(format!("**Suggestion:** {footer}\n\n"));
    }
    Ok((title, body_parts))
}

/// Title/body from piped stdin (e.g., `echo "bug report" | tool feedback bug`).
fn title_and_body_from_stdin(kind: &str) -> Result<(String, Vec<String>), String> {
    use std::io::{IsTerminal, Read};
    if std::io::stdin().is_terminal() {
        return Err(NO_CONTENT_ERR.to_string());
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    if input.trim().is_empty() {
        return Err(NO_CONTENT_ERR.to_string());
    }
    let (title, description) = stdin_title_and_body(kind, &input);
    let body_parts = if description.is_empty() {
        Vec::new()
    } else {
        vec![description]
    };
    Ok((title, body_parts))
}

const NO_CONTENT_ERR: &str =
    "No issue content specified. Use --from-last-error or pipe content into stdin.";

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
