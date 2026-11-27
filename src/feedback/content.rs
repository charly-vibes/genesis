//! Issue title and body content derivation.
//!
//! Splits the content sourcing for [`crate::feedback::handle_feedback`]
//! out of the module root: the last scratch error (`--from-last-error`)
//! or piped stdin are turned into an issue title plus body sections.

use super::FeedbackArgs;
use super::scratch;

const NO_CONTENT_ERR: &str =
    "No issue content specified. Use --from-last-error or pipe content into stdin.";

/// Maximum captured-message length rendered under `## Error` (chars).
const MAX_MESSAGE_LEN: usize = 4000;

/// Truncation marker appended when the captured message exceeds the cap.
const TRUNCATION_MARKER: &str = "\n[… message truncated — original exceeded 4000 chars …]";

/// Render `message` inside a fenced code block whose fence is one backtick
/// longer than the longest backtick run in the (capped) message, so a
/// message containing ``` cannot terminate the fence early (CommonMark
/// fence rule). The message is capped at [`MAX_MESSAGE_LEN`] chars with an
/// explicit truncation marker.
pub(super) fn fenced_code_block(message: &str) -> String {
    let (message, truncated) = match message.char_indices().nth(MAX_MESSAGE_LEN) {
        Some((byte_idx, _)) => (&message[..byte_idx], true),
        None => (message, false),
    };
    let longest_run = message
        .split(|c| c != '`')
        .map(|run| run.len())
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest_run.max(2) + 1);
    let mut out = format!("{fence}\n{message}\n{fence}\n\n");
    if truncated {
        out.push_str(TRUNCATION_MARKER);
        out.push_str("\n\n");
    }
    out
}

/// Derived issue content plus the scratch record fields (if any) that the
/// `## Environment` bundle needs, so [`super::handle_feedback`] can thread
/// command/exit/footer into context gathering instead of discarding them.
pub(super) struct IssueContent {
    pub title: String,
    pub body_parts: Vec<String>,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub suggestion_footer: Option<String>,
}

/// Derive the issue title and body sections from the content source:
/// the last scratch error (`--from-last-error`) or piped stdin.
///
/// The scratch record's command, exit code, and footer are returned alongside
/// the body so the environment bundle can include them (wai reference wiring).
pub(super) fn issue_title_and_body(
    args: &FeedbackArgs,
    tool_name: &str,
) -> Result<IssueContent, String> {
    if args.from_last_error {
        title_and_body_from_error(&args.kind, tool_name)
    } else {
        title_and_body_from_stdin(&args.kind)
    }
}

/// Title/body from the last recorded error for `tool_name`.
fn title_and_body_from_error(kind: &str, tool_name: &str) -> Result<IssueContent, String> {
    let Some(record) = scratch::read_last_error(tool_name) else {
        return Err(format!(
            "No recent error recorded for '{}'. Run a command that produces an error first.",
            tool_name
        ));
    };
    let cmd_str = record.argv.join(" ");
    Ok(IssueContent {
        title: format!("[{kind}] auto-reported error: {cmd_str}"),
        body_parts: error_body_parts(&record, &cmd_str),
        command: Some(cmd_str),
        exit_code: Some(record.exit),
        suggestion_footer: record.footer.clone(),
    })
}

/// Markdown body for an auto-reported error: command, exit code, the
/// captured message as a fenced block, and the suggestion footer when
/// present.
fn error_body_parts(record: &scratch::ErrorRecord, cmd_str: &str) -> Vec<String> {
    let mut body_parts = vec!["## Error\n\n".to_string()];
    body_parts.push(format!("**Command:** `{cmd_str}`\n\n"));
    body_parts.push(format!("**Exit code:** {}\n\n", record.exit));
    if let Some(message) = &record.message {
        body_parts.push(fenced_code_block(message));
    }
    if let Some(footer) = &record.footer {
        body_parts.push(format!("**Suggestion:** {footer}\n\n"));
    }
    body_parts
}

/// Piped stdin for a feedback report; `None` when the caller is on a
/// TTY or the pipe carries nothing (nothing to report).
fn read_piped_stdin() -> Option<String> {
    use std::io::{IsTerminal, Read};
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    (!input.trim().is_empty()).then_some(input)
}

/// Title/body from piped stdin (e.g., `echo "bug report" | tool feedback bug`).
/// No scratch record exists, so the environment fields stay `None`.
fn title_and_body_from_stdin(kind: &str) -> Result<IssueContent, String> {
    let Some(input) = read_piped_stdin() else {
        return Err(NO_CONTENT_ERR.to_string());
    };
    let (title, description) = stdin_title_and_body(kind, &input);
    Ok(IssueContent {
        title,
        body_parts: if description.is_empty() {
            Vec::new()
        } else {
            vec![description]
        },
        command: None,
        exit_code: None,
        suggestion_footer: None,
    })
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
