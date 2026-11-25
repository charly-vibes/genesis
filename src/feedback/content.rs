//! Issue title and body content derivation.
//!
//! Splits the content sourcing for [`crate::feedback::handle_feedback`]
//! out of the module root: the last scratch error (`--from-last-error`)
//! or piped stdin are turned into an issue title plus body sections.

use super::FeedbackArgs;
use super::scratch;

const NO_CONTENT_ERR: &str =
    "No issue content specified. Use --from-last-error or pipe content into stdin.";

/// Derive the issue title and body sections from the content source:
/// the last scratch error (`--from-last-error`) or piped stdin.
pub(super) fn issue_title_and_body(
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
    if let Some(footer) = &record.footer {
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

/// Partition piped stdin into an issue title and a Description body.
///
/// - Single-line input (including a trailing newline, e.g. from `echo`)
///   keeps the generic `[{kind}] feedback report` title and lands verbatim
///   in the Description — backward compatible with single-line pipelines.
/// - Multi-line input promotes the first line to the title and uses the
///   remainder as the Description. The full input is read, so multi-
///   paragraph reports are never truncated.
pub(super) fn stdin_title_and_body(kind: &str, input: &str) -> (String, String) {
    let trimmed = input.trim();
    if let Some((first, rest)) = trimmed.split_once('\n')
        && let Some(split) = multiline_split(kind, first, rest)
    {
        return split;
    }
    (
        format!("[{kind}] feedback report"),
        format!("## Description\n\n{trimmed}\n\n"),
    )
}

/// The `[first line]` title + Description body for non-empty first lines.
/// `None` falls back to the generic single-line title.
fn multiline_split(kind: &str, first: &str, rest: &str) -> Option<(String, String)> {
    let first = first.trim();
    let rest = rest.trim();
    if first.is_empty() {
        return None;
    }
    let title = format!("[{kind}] {first}");
    let body = if rest.is_empty() {
        String::new()
    } else {
        format!("## Description\n\n{rest}\n\n")
    };
    Some((title, body))
}

/// Apply a user-supplied title override, keeping the `[{kind}] ` prefix.
/// `None` keeps the derived title unchanged.
pub(super) fn apply_title_override(kind: &str, derived: &str, title: Option<&str>) -> String {
    match title {
        Some(t) => format!("[{kind}] {t}"),
        None => derived.to_string(),
    }
}
