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

use crate::evals::{CheckOutcome, EnvelopeOutcome, Scenario};
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

/// Valid issue kinds.
const VALID_KINDS: &[&str] = &["bug", "feature", "question", "chore"];

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
        // Try reading from stdin (piped input, e.g., `echo "bug report" | tool feedback bug`)
        use std::io::{IsTerminal, Read};
        if !std::io::stdin().is_terminal() {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).ok();
            if input.trim().is_empty() {
                return Err(
                    "No issue content specified. Use --from-last-error or pipe content into stdin."
                        .to_string(),
                );
            }
            let (derived_title, description) = stdin_title_and_body(&args.kind, &input);
            title = derived_title;
            if !description.is_empty() {
                body_parts.push(description);
            }
        } else {
            return Err(
                "No issue content specified. Use --from-last-error or pipe content into stdin."
                    .to_string(),
            );
        }
    }

    // User-supplied title wins over every derived title.
    title = apply_title_override(&args.kind, &title, args.title.as_deref());

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

    /// Unique tool name per test to avoid races with parallel tests.
    fn test_tool(label: &str) -> String {
        format!("test-{}-{}", label, std::process::id())
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
        let tool = test_tool("dry-run");
        write_scratch(&tool, 1);

        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(result.is_ok(), "dry run should succeed: {:?}", result);
    }

    #[test]
    fn test_handle_feedback_from_last_error_no_scratch() {
        let dir = tmp();
        let tool = test_tool("no-scratch");
        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should fail when no scratch exists");
        assert!(
            result.unwrap_err().contains("No recent error"),
            "should mention missing scratch"
        );
    }

    #[test]
    fn test_handle_feedback_no_from_last_error() {
        let dir = tmp();
        let tool = test_tool("no-from-last");
        let args = FeedbackArgs::new("bug", true, false);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should fail without --from-last-error");
        assert!(
            result.unwrap_err().contains("--from-last-error"),
            "should suggest --from-last-error"
        );
    }

    #[test]
    fn test_handle_feedback_invalid_kind() {
        let dir = tmp();
        let tool = test_tool("invalid-kind");
        let args = FeedbackArgs::new("invalid-kind", true, true);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "should reject invalid kind");
        let err = result.unwrap_err();
        assert!(err.contains("unknown kind"), "should say 'unknown kind'");
    }

    #[test]
    fn test_handle_feedback_typo_suggestion_for_kind() {
        let dir = tmp();
        let tool = test_tool("typo");
        let args = FeedbackArgs::new("featuer", true, true);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
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
            let tool = test_tool("valid-kind");
            write_scratch(&tool, 1);
            let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
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
        let tool = test_tool("dry-run-2");
        write_scratch(&tool, 1);
        let args = FeedbackArgs::new("bug", true, true);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        match result {
            Ok(gh::GhResult::FallbackUrl(url)) => {
                assert!(url.contains("github.com"), "URL should contain github.com");
            }
            Ok(other) => panic!("expected FallbackUrl, got {:?}", other),
            Err(e) => panic!("should not error: {}", e),
        }
    }

    // --- stdin title/body partitioning (genesis-gle, genesis-og6) ---

    #[test]
    fn stdin_single_line_keeps_generic_title() {
        let (title, body) = stdin_title_and_body("bug", "summary line");
        assert_eq!(title, "[bug] feedback report");
        assert!(body.contains("summary line"), "body must carry the input");
    }

    #[test]
    fn stdin_trailing_newline_is_still_single_line() {
        // `echo "x" | tool feedback bug` produces "x\n" — must keep the
        // generic title, not treat the trailing newline as a second line.
        let (title, body) = stdin_title_and_body("bug", "summary line\n");
        assert_eq!(title, "[bug] feedback report");
        assert!(body.contains("summary line"));
    }

    #[test]
    fn stdin_multiline_promotes_first_line_to_title() {
        let input = "Bare row-id references fail when intent id has a dot\n\nThe resolution algorithm is undocumented. Details follow.\n";
        let (title, body) = stdin_title_and_body("bug", input);
        assert_eq!(
            title,
            "[bug] Bare row-id references fail when intent id has a dot"
        );
        assert!(body.contains("The resolution algorithm is undocumented."));
        assert!(
            !body.contains("Bare row-id references"),
            "first line must not be duplicated into the body"
        );
        assert!(body.starts_with("## Description"));
    }

    #[test]
    fn stdin_multiline_blank_lines_around_single_line_is_still_single() {
        // A title line followed by blank lines trims to one line — the
        // echo-compat rule wins over title promotion.
        let (title, body) = stdin_title_and_body("feature", "add a flag\n\n");
        assert_eq!(title, "[feature] feedback report");
        assert!(body.contains("add a flag"));
    }

    #[test]
    fn stdin_multiline_title_plus_body_omits_first_line_from_body() {
        let (title, body) = stdin_title_and_body("feature", "add a flag\n\nbody text\n");
        assert_eq!(title, "[feature] add a flag");
        assert!(body.contains("body text"));
        assert!(!body.contains("add a flag"));
    }

    #[test]
    fn title_override_wins_over_derived_and_first_line() {
        assert_eq!(
            apply_title_override("bug", "[bug] feedback report", Some("My title")),
            "[bug] My title"
        );
        assert_eq!(
            apply_title_override(
                "bug",
                "[bug] auto-reported error: ah check",
                Some("My title")
            ),
            "[bug] My title"
        );
        assert_eq!(
            apply_title_override("bug", "[bug] feedback report", None),
            "[bug] feedback report",
            "no override keeps the derived title"
        );
    }

    #[test]
    fn with_title_builder_sets_override() {
        let args = FeedbackArgs::new("bug", true, false).with_title("My title");
        assert_eq!(args.title.as_deref(), Some("My title"));
        // ::new keeps the 3-arg signature — no override by default.
        let plain = FeedbackArgs::new("bug", true, false);
        assert_eq!(plain.title, None);
    }
}

// ---------------------------------------------------------------------------
// Feedback → Scenario conversion (add-aix-eval-loop §4, design D4)
// ---------------------------------------------------------------------------

/// Errors from feedback → scenario conversion.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// No scratch `ErrorRecord` exists for the tool — nothing to convert.
    #[error("no scratch error record for tool `{0}`")]
    NoScratchRecord(String),
}

/// Extract the suggested command from a `→ Run: …` footer line, if any.
fn footer_hint_command(footer: &str) -> Option<String> {
    footer
        .lines()
        .find_map(|l| l.split_once("Run: "))
        .map(|(_, cmd)| cmd.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl Scenario {
    /// Convert captured feedback context into a replayable regression
    /// scenario (design D4).
    ///
    /// # Examples
    ///
    /// ```
    /// use genesis::evals::{AgentStep, Scenario};
    /// use genesis::feedback::context::ContextBundle;
    ///
    /// // The bundle a tool captured when the user ran `feedback`
    /// // (the "aix-gap" failure: sync exited 2 with a repair hint).
    /// let bundle = ContextBundle {
    ///     tool_name: "my-tool".into(),
    ///     tool_version: "1.0.0".into(),
    ///     command: Some("my-tool sync".into()),
    ///     exit_code: Some(2),
    ///     suggestion_footer: Some("state file corrupt\n  \u{2192} Run: my-tool repair".into()),
    ///     os_arch: "linux/x86_64".into(),
    ///     shell: None,
    ///     gh_version: None,
    ///     git_remote: None,
    ///     git_branch: None,
    ///     git_dirty: None,
    ///     repo_state: vec![],
    ///     repro_hash: 0x1234,
    /// };
    ///
    /// // The agent-reported failure becomes a regression scenario.
    /// let scenario = Scenario::from_feedback_context(&bundle, vec![]);
    ///
    /// // Replayed against the recorded transcript — in-process, no LLM,
    /// // no subprocess.
    /// let recorded = AgentStep {
    ///     command: "my-tool sync".into(),
    ///     stdout: r#"{"ok": false, "data": {"code": "E_SYNC"},
    ///         "hints": [{"command": "my-tool repair"}]}"#.into(),
    ///     stderr: String::new(),
    ///     exit_code: 2,
    ///     executed: true,
    /// };
    /// let report = scenario.run(vec![recorded]).expect("fixture");
    /// assert!(report.passed);
    /// ```
    ///
    /// The scenario embeds the recorded failure signature: the recorded
    /// command, exit code, and an expectation that the error envelope
    /// carries the suggestion footer's hint. Callers replay it against
    /// recorded [`crate::evals::AgentStep`] transcripts — conversion and
    /// replay are pure in-process computation (no LLM, no subprocess).
    ///
    /// `fixtures` are caller-supplied `(path, content)` pairs: the
    /// converting tool knows which files were in play; genesis must not
    /// re-snapshot the working tree. Bundle-only conversion (empty list)
    /// yields a prompt-only scenario.
    pub fn from_feedback_context(
        bundle: &context::ContextBundle,
        fixtures: Vec<(String, String)>,
    ) -> Self {
        let recorded_command = bundle.command.clone();
        let recorded_exit = bundle.exit_code;
        let hint = bundle
            .suggestion_footer
            .as_deref()
            .and_then(footer_hint_command);
        let footer_text = bundle.suggestion_footer.clone().unwrap_or_default();

        Self::new(
            format!("feedback-{:x}", bundle.repro_hash),
            match &recorded_command {
                Some(cmd) => format!(
                    "Reproduce and diagnose: `{cmd}` exited {}. {footer_text}",
                    recorded_exit.unwrap_or_default()
                ),
                None => format!("Reproduce and diagnose the recorded failure. {footer_text}"),
            },
        )
        .fixture_files_from(fixtures)
        .check("reproduces-recorded-failure", move |result| {
            let Some(step) = result.steps.first() else {
                return CheckOutcome::tool_fault("no steps in replay");
            };
            if let Some(cmd) = &recorded_command
                && step.command != *cmd
            {
                return CheckOutcome::tool_fault(format!(
                    "recorded command is `{cmd}` but replay ran `{}`",
                    step.command
                ));
            }
            if let Some(exit) = recorded_exit
                && step.exit_code != exit
            {
                return CheckOutcome::tool_fault(format!(
                    "recorded exit code is {exit} but replay exited {}",
                    step.exit_code
                ));
            }
            match crate::evals::parse_envelope(&step.stdout) {
                Ok(EnvelopeOutcome::Error { hint_commands, .. }) => {
                    if let Some(hint) = &hint
                        && !hint_commands.contains(hint)
                    {
                        return CheckOutcome::tool_fault(format!(
                            "error envelope hints {hint_commands:?} lack the footer's suggested command `{hint}`"
                        ));
                    }
                    CheckOutcome::pass()
                }
                Ok(EnvelopeOutcome::Ok) => {
                    CheckOutcome::tool_fault("replay shows ok:true; expected the recorded error envelope")
                }
                Err(e) => CheckOutcome::tool_fault(format!("step 0: {e}")),
                }
            })
    }
}

/// Convert the last scratch error record for `tool_name` into a regression
/// scenario (design D4). Convenience wrapper around
/// [`Scenario::from_feedback_context`] for the `--from-last-error` path.
///
/// Returns [`ConversionError::NoScratchRecord`] when no record exists —
/// a typed error, not a panic.
pub fn from_last_error(
    tool_name: &str,
    fixtures: Vec<(String, String)>,
) -> Result<Scenario, ConversionError> {
    let record = scratch::read_last_error(tool_name)
        .ok_or_else(|| ConversionError::NoScratchRecord(tool_name.to_string()))?;
    let bundle = context::ContextBundle {
        tool_name: tool_name.to_string(),
        tool_version: "unknown".to_string(),
        command: Some(record.argv.join(" ")),
        exit_code: Some(record.exit),
        suggestion_footer: record.footer,
        os_arch: "unknown/unknown".to_string(),
        shell: None,
        gh_version: None,
        git_remote: None,
        git_branch: None,
        git_dirty: None,
        repo_state: vec![],
        // The record's kind + timestamp stand in for a repro hash: stable
        // per failure signature, traceable back to the scratch file.
        repro_hash: {
            use std::hash::{BuildHasher, Hasher};
            let mut h =
                std::hash::BuildHasherDefault::<std::hash::DefaultHasher>::default().build_hasher();
            h.write(record.kind.as_bytes());
            h.write(record.ts.as_bytes());
            h.finish()
        },
    };
    Ok(Scenario::from_feedback_context(&bundle, fixtures))
}
