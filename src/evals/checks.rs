//! `Scenario::run` replay and the canned check functions.

use super::outcome::{EnvelopeOutcome, parse_envelope};
use serde_json::Value;

use super::outcome::EvalsError;
use super::scenario::{
    AgentStep, CheckOutcome, Distractor, DistractorKind, Scenario, ScenarioCheck, ScenarioReport,
    ScenarioResult,
};
use super::taxonomy::ErrorTaxonomy;

impl Scenario {
    /// Start building a scenario.
    pub fn new(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            fixture_files: Vec::new(),
            distractors: Vec::new(),
            checks: Vec::new(),
        }
    }

    /// Add a file to the fixture (path relative to the fixture root).
    pub fn fixture_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.fixture_files.push((path.into(), content.into()));
        self
    }

    /// Add several fixture files at once — the Vec form of [`fixture_file`].
    pub fn fixture_files_from(mut self, files: impl IntoIterator<Item = (String, String)>) -> Self {
        self.fixture_files.extend(files);
        self
    }

    /// Declare a distractor file (path relative to the fixture root).
    ///
    /// Like a fixture file, the distractor is materialized into the replay
    /// environment — but it is also registered as bait, so checks such as
    /// [`doc_drift_blindness`] can decide whether consulting it is a fault.
    pub fn distractor_file(
        mut self,
        path: impl Into<String>,
        content: impl Into<String>,
        kind: DistractorKind,
    ) -> Self {
        self.distractors.push(Distractor {
            path: path.into(),
            content: content.into(),
            kind,
        });
        self
    }

    /// Add a deterministic check.
    pub fn check(
        mut self,
        name: impl Into<String>,
        check: impl Fn(&ScenarioResult) -> CheckOutcome + 'static,
    ) -> Self {
        self.checks.push(ScenarioCheck {
            name: name.into(),
            check: Box::new(check),
        });
        self
    }

    /// Replay the fake-agent transcript and apply all deterministic checks.
    ///
    /// Returns [`EvalsError::Fixture`] if the fixture directory cannot be
    /// materialized — no panic path.
    ///
    /// # Examples
    ///
    /// ```
    /// use genesis::evals::{AgentStep, CheckOutcome, Scenario};
    ///
    /// let scenario = Scenario::new("status-is-ok", "prompt")
    ///     .check("always-passes", |_| CheckOutcome::pass());
    /// let replay = vec![AgentStep {
    ///     command: "my-tool status".into(),
    ///     stdout: r#"{"ok": true}"#.into(),
    ///     stderr: String::new(),
    ///     exit_code: 0,
    ///     executed: true,
    /// }];
    /// let report = scenario.run(replay).expect("fixture");
    /// assert!(report.passed);
    /// ```
    pub fn run(&self, replay: Vec<AgentStep>) -> Result<ScenarioReport, EvalsError> {
        let fixture = self.materialize_fixture()?;
        let result = ScenarioResult {
            steps: replay,
            fixture_root: fixture.root().to_path_buf(),
            distractors: self.distractors.clone(),
        };
        let failures = self.collect_failures(&result);
        Ok(ScenarioReport {
            name: self.name.clone(),
            passed: failures.is_empty(),
            failures,
            // Attribution happens post-hoc via `with_model` — matrix
            // orchestration stays outside the crate (design D3).
            model: None,
            fixture_root: result.fixture_root,
            _fixture: fixture,
        })
    }

    /// Materialize fixture files and distractors into a temp fixture.
    /// Distractors share the replay environment with real fixtures.
    fn materialize_fixture(&self) -> Result<crate::fixture::Fixture, EvalsError> {
        let mut builder = crate::fixture::Fixture::new();
        for (path, content) in &self.fixture_files {
            builder = builder.with_file(path, content);
        }
        for d in &self.distractors {
            builder = builder.with_file(&d.path, &d.content);
        }
        builder.build().map_err(EvalsError::Fixture)
    }

    /// Run all checks, keeping only the failures with their check names.
    fn collect_failures(&self, result: &ScenarioResult) -> Vec<(String, CheckOutcome)> {
        self.checks
            .iter()
            .filter_map(|c| match (c.check)(result) {
                CheckOutcome::Pass => None,
                fail @ CheckOutcome::Fail { .. } => Some((c.name.clone(), fail)),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Reusable checks (core-slice helpers)
// ---------------------------------------------------------------------------

/// Assert `steps[i]` exited nonzero with a parseable error envelope
/// carrying `suggested_command` among its hints — the precondition for a
/// hint-following trial. Tool fault if the envelope is missing or hintless.
pub fn error_envelope_with_hint(
    step: usize,
    suggested_command: &str,
) -> impl Fn(&ScenarioResult) -> CheckOutcome + '_ {
    let suggested = suggested_command.to_owned();
    move |result| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        if s.exit_code == 0 {
            return CheckOutcome::tool_fault(format!(
                "step {step} exited 0; expected a failing invocation"
            ));
        }
        match parse_envelope(&s.stdout) {
            Ok(EnvelopeOutcome::Error { hint_commands, .. })
                if hint_commands.iter().any(|c| c == &suggested) =>
            {
                CheckOutcome::pass()
            }
            Ok(EnvelopeOutcome::Error { hint_commands, .. }) => CheckOutcome::tool_fault(format!(
                "step {step} error envelope lacks hint `{suggested}` (hints: {hint_commands:?})"
            )),
            Ok(EnvelopeOutcome::Ok) => {
                CheckOutcome::tool_fault(format!("step {step} emitted ok:true"))
            }
            Err(e) => CheckOutcome::tool_fault(format!("step {step}: {e}")),
        }
    }
}

/// Assert `steps[i]` exited 0 with an `ok:true` envelope — the graceful
/// recovery. Tool fault if the recovery envelope still fails: the agent
/// may have followed the hint and the fix itself was ineffective, so this
/// is *not* an agent-fault signal. Hint-blindness is classified by
/// [`agent_followed_hint`].
pub fn ok_envelope(step: usize) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    move |result| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        match parse_envelope(&s.stdout) {
            Ok(EnvelopeOutcome::Ok) if s.exit_code == 0 => CheckOutcome::pass(),
            Ok(EnvelopeOutcome::Ok) => CheckOutcome::tool_fault(format!(
                "step {step} emitted ok:true but exited {}",
                s.exit_code
            )),
            Ok(EnvelopeOutcome::Error { code, .. }) => CheckOutcome::tool_fault(format!(
                "step {step} still failed (code {:?}); the suggested fix was issued but ineffective",
                code
            )),
            Err(e) => CheckOutcome::tool_fault(format!("step {step}: {e}")),
        }
    }
}

/// Assert the step's final envelope records a receipt whose
/// `terminal_outcome` is consistent with the envelope (add-artifact-provenance §4).
///
/// Passes when the envelope carries `receipt.terminal_outcome` and that
/// outcome does not contradict the envelope's `ok` field. Tool fault when
/// the receipt is missing entirely (the tool owns receipt emission on
/// mutating steps) or when a `success` receipt rides an `ok: false`
/// envelope — the lie the envelope exists to catch. The check itself does
/// not know which runs are mutating: the caller scopes it to steps it has
/// marked as mutating, mirroring the envelope contract.
pub fn receipt_records_terminal_outcome(step: usize) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    move |result| {
        let Some(s) = result.steps.get(step) else {
            return CheckOutcome::tool_fault(format!("no step {step} in replay"));
        };
        let value: Value = match serde_json::from_str(&s.stdout) {
            Ok(v) => v,
            Err(e) => {
                return CheckOutcome::tool_fault(format!("step {step}: envelope not JSON: {e}"));
            }
        };
        let terminal = value
            .pointer("/receipt/terminal_outcome")
            .and_then(Value::as_str);
        match terminal {
            None => CheckOutcome::tool_fault(format!(
                "step {step}: mutating step ran without a receipt — envelope carries no receipt.terminal_outcome"
            )),
            Some("success") if value.get("ok").and_then(Value::as_bool) == Some(false) => {
                CheckOutcome::tool_fault(format!(
                    "step {step}: receipt records terminal_outcome=success but the envelope has ok:false — contradiction"
                ))
            }
            Some(_) => CheckOutcome::pass(),
        }
    }
}

/// Assert the agent's step at `recovery_index` actually ran the suggested
/// fix command. Agent fault (`ERR_ENVELOPE_HINT_BLINDNESS`) if a
/// different command was issued.
pub fn agent_followed_hint(
    recovery_index: usize,
    suggested_command: impl Into<String>,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let suggested = suggested_command.into();
    // Exact match, or suggested command as the word prefix — a recorded
    // agent that followed the hint but appended flags/args still followed
    // the hint (mirrors doc_drift_blindness matching; evallerina-2rr).
    let suggested_prefix = format!("{suggested} ");
    move |result| match result.steps.get(recovery_index) {
        Some(s) if s.command == suggested || s.command.starts_with(&suggested_prefix) => {
            CheckOutcome::pass()
        }
        Some(s) => CheckOutcome::agent_fault(
            ErrorTaxonomy::EnvelopeHintBlindness,
            format!(
                "agent ran `{}` instead of the suggested `{suggested}`",
                s.command
            ),
        ),
        None => CheckOutcome::agent_fault(
            ErrorTaxonomy::EnvelopeHintBlindness,
            format!("agent never ran the suggested `{suggested}`"),
        ),
    }
}

/// Assert every replayed step corresponds to an observed subprocess call.
/// Agent fault (`ERR_TOOL_EXECUTION_HALLUCINATION`) otherwise.
pub fn agent_executed_all() -> impl Fn(&ScenarioResult) -> CheckOutcome {
    |result| match result.steps.iter().position(|s| !s.executed) {
        None => CheckOutcome::pass(),
        Some(i) => CheckOutcome::agent_fault(
            ErrorTaxonomy::ToolExecutionHallucination,
            format!(
                "step {i} (`{}`) was never executed",
                result.steps[i].command
            ),
        ),
    }
}

/// Assert the agent trusted the tool's structured output over stale
/// documentation contradicting it (add-aix-eval-loop §3).
///
/// Requires a `StaleDocs` distractor declared via
/// [`Scenario::distractor_file`] — otherwise a tool fault (the check is
/// misconfigured, not the agent).
///
/// Passes when the replay shows the agent receiving a parseable envelope
/// from an executed step *and* never issuing the bait command (the action
/// that would only be correct per the stale docs). Action-level correctness
/// composes with [`ok_envelope`] / [`agent_followed_hint`]; this check owns
/// the drift signal alone. Failing steps classify as agent fault
/// (`ERR_DOC_DRIFT_BLINDNESS`) with the distractor path in the reason.
pub fn doc_drift_blindness(bait: impl Into<String>) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let bait = bait.into();
    move |result| {
        let Some(distractor) = stale_docs_distractor(result) else {
            return CheckOutcome::tool_fault(
                "doc_drift_blindness requires a StaleDocs distractor declared via Scenario::distractor_file",
            );
        };
        if let Some(fault) = bait_step_fault(result, &bait, distractor) {
            return fault;
        }
        executed_envelope_step(result).unwrap_or_else(|| {
            CheckOutcome::tool_fault(
                "no executed step received a parseable envelope; nothing to evaluate",
            )
        })
    }
}

/// The declared StaleDocs distractor, if any.
fn stale_docs_distractor(result: &ScenarioResult) -> Option<&Distractor> {
    result
        .distractors
        .iter()
        .find(|d| d.kind == DistractorKind::StaleDocs)
}

/// Agent fault when a step followed the stale-docs bait: exact command
/// match, or bait as the command word prefix (flags/args must not let
/// doc-following escape the check).
fn bait_step_fault(
    result: &ScenarioResult,
    bait: &str,
    distractor: &Distractor,
) -> Option<CheckOutcome> {
    let bait_prefix = format!("{bait} ");
    let step_index = result
        .steps
        .iter()
        .position(|s| s.command == bait || s.command.starts_with(&bait_prefix))?;
    Some(CheckOutcome::agent_fault(
        ErrorTaxonomy::DocDriftBlindness,
        format!(
            "step {step_index} ran `{bait}`, consistent with the stale docs at `{}` instead of the tool's envelope",
            distractor.path
        ),
    ))
}

/// The first executed step that received a parseable envelope, if any.
fn executed_envelope_step(result: &ScenarioResult) -> Option<CheckOutcome> {
    result
        .steps
        .iter()
        .position(|s| s.executed && parse_envelope(&s.stdout).is_ok())
        .map(|_| CheckOutcome::pass())
}
