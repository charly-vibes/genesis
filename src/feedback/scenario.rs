//! Feedback → Scenario conversion (add-aix-eval-loop §4, design D4).

use super::context;
use super::scratch;
use crate::evals::{CheckOutcome, EnvelopeOutcome, Scenario};

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
            verify_replayed_step(result, &recorded_command, recorded_exit, hint.as_deref())
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
    let repro_hash = record_repro_hash(&record);
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
        repro_hash,
    };
    Ok(Scenario::from_feedback_context(&bundle, fixtures))
}

/// Verify a replayed step against the recorded failure signature.
fn verify_replayed_step(
    result: &crate::evals::ScenarioResult,
    recorded_command: &Option<String>,
    recorded_exit: Option<i32>,
    hint: Option<&str>,
) -> CheckOutcome {
    let Some(step) = result.steps.first() else {
        return CheckOutcome::tool_fault("no steps in replay");
    };
    if let Some(fault) = command_mismatch(&step.command, recorded_command) {
        return fault;
    }
    if let Some(fault) = exit_mismatch(step.exit_code, recorded_exit) {
        return fault;
    }
    match crate::evals::parse_envelope(&step.stdout) {
        Ok(EnvelopeOutcome::Error { hint_commands, .. }) => {
            match_envelope_hints(hint, &hint_commands)
        }
        Ok(EnvelopeOutcome::Ok) => {
            CheckOutcome::tool_fault("replay shows ok:true; expected the recorded error envelope")
        }
        Err(e) => CheckOutcome::tool_fault(format!("step 0: {e}")),
    }
}

/// The error envelope's hints must carry the footer's suggested command.
fn match_envelope_hints(hint: Option<&str>, hint_commands: &[String]) -> CheckOutcome {
    if let Some(hint) = hint
        && !hint_commands.contains(&hint.to_string())
    {
        return CheckOutcome::tool_fault(format!(
            "error envelope hints {hint_commands:?} lack the footer's suggested command `{hint}`"
        ));
    }
    CheckOutcome::pass()
}

/// Stable per-failure-signature hash of a scratch record's kind + timestamp.
fn record_repro_hash(record: &scratch::ErrorRecord) -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::hash::BuildHasherDefault::<std::hash::DefaultHasher>::default().build_hasher();
    h.write(record.kind.as_bytes());
    h.write(record.ts.as_bytes());
    h.finish()
}

/// Fault when the replay ran a different command than recorded.
fn command_mismatch(ran: &str, recorded_command: &Option<String>) -> Option<CheckOutcome> {
    let cmd = recorded_command.as_ref()?;
    if ran == cmd {
        return None;
    }
    Some(CheckOutcome::tool_fault(format!(
        "recorded command is `{cmd}` but replay ran `{ran}`"
    )))
}

/// Fault when the replay exited with a different code than recorded.
fn exit_mismatch(ran: i32, recorded_exit: Option<i32>) -> Option<CheckOutcome> {
    let exit = recorded_exit?;
    if ran == exit {
        return None;
    }
    Some(CheckOutcome::tool_fault(format!(
        "recorded exit code is {exit} but replay exited {ran}"
    )))
}
