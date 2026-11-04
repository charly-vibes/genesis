//! Deterministic eval primitives for agentic CLI tools.
//!
//! Tools in the charly-vibes suite each hand-roll the same evaluation
//! scaffolding: fixture setup, envelope assertions over captured output,
//! and a way to say *why* a scenario failed. This module is the shared
//! core slice:
//!
//! - [`ErrorTaxonomy`] — the six cross-tool failure classifications
//!   (`ERR_*` codes) from the evaluation-framework research.
//! - Envelope-assertion helpers ([`parse_envelope`]) over captured
//!   subprocess stdout — the portable assertion ("exited nonzero with a
//!   parseable error envelope") holds regardless of exit-code refinement.
//! - [`Scenario`] — fixture setup + agent prompt + deterministic checks,
//!   replayed against a fake-agent transcript ([`AgentStep`]). No live
//!   LLM calls, no subprocess execution environment.
//!
//! Managed-block audit utilities and AIX-ablation provisioning are
//! follow-up slices (see genesis-zxv).

use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Error taxonomy
// ---------------------------------------------------------------------------

/// Structured classification of an eval-trial failure.
///
/// Codes follow the evaluation-framework research; each variant maps to
/// one `ERR_*` string so harnesses can emit machine-readable signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorTaxonomy {
    /// Tool returned `ok: false` with a populated `hints` vector, but the
    /// agent's subsequent command ignored the suggested fix.
    EnvelopeHintBlindness,
    /// Agent executed an illegal state transition or bypassed state checks
    /// by manually editing local state files.
    StateMachineViolation,
    /// Agent overwrote or deleted AGENTS.md managed-block tags, disrupting
    /// future tool injection.
    ManagedBlockCorruption,
    /// Agent produced text output claiming to have run a command without a
    /// corresponding subprocess call in the terminal environment.
    ToolExecutionHallucination,
    /// Agent encountered an unexpected error but failed to refresh its
    /// project orientation context (e.g. skipped `prime`/`status`).
    ContextRecoveryFailure,
    /// Agent defaulted to a blunt strategy, ignoring the specialized tool
    /// the environment provides (e.g. full test suite instead of test
    /// selection).
    ToolDiscoveryFailure,
    /// Agent acted on stale documentation instead of the tool's structured
    /// output contradicting it (add-aix-eval-loop §3, corpus B171/B128).
    DocDriftBlindness,
    /// Live-tier agent failed to produce a parseable structured action
    /// (`command` + `done`) after one re-ask — the re-ask consumes a turn
    /// (add-evals-guidelines, live action protocol). Distinct from
    /// `ToolExecutionHallucination`, which stays a replay-tier code for
    /// claims of runs that never happened.
    ActionFormatViolation,
}

impl ErrorTaxonomy {
    /// The machine-readable `ERR_*` code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EnvelopeHintBlindness => "ERR_ENVELOPE_HINT_BLINDNESS",
            Self::StateMachineViolation => "ERR_STATE_MACHINE_VIOLATION",
            Self::ManagedBlockCorruption => "ERR_MANAGED_BLOCK_CORRUPTION",
            Self::ToolExecutionHallucination => "ERR_TOOL_EXECUTION_HALLUCINATION",
            Self::ContextRecoveryFailure => "ERR_CONTEXT_RECOVERY_FAILURE",
            Self::ToolDiscoveryFailure => "ERR_TOOL_DISCOVERY_FAILURE",
            Self::DocDriftBlindness => "ERR_DOC_DRIFT_BLINDNESS",
            Self::ActionFormatViolation => "ERR_ACTION_FORMAT_VIOLATION",
        }
    }

    /// Parse an `ERR_*` code back into a variant.
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "ERR_ENVELOPE_HINT_BLINDNESS" => Self::EnvelopeHintBlindness,
            "ERR_STATE_MACHINE_VIOLATION" => Self::StateMachineViolation,
            "ERR_MANAGED_BLOCK_CORRUPTION" => Self::ManagedBlockCorruption,
            "ERR_TOOL_EXECUTION_HALLUCINATION" => Self::ToolExecutionHallucination,
            "ERR_CONTEXT_RECOVERY_FAILURE" => Self::ContextRecoveryFailure,
            "ERR_TOOL_DISCOVERY_FAILURE" => Self::ToolDiscoveryFailure,
            "ERR_DOC_DRIFT_BLINDNESS" => Self::DocDriftBlindness,
            "ERR_ACTION_FORMAT_VIOLATION" => Self::ActionFormatViolation,
            _ => return None,
        })
    }

    /// One-line, agent-actionable description of the failure mode.
    pub fn description(&self) -> &'static str {
        match self {
            Self::EnvelopeHintBlindness => {
                "the tool suggested a fix in its error envelope, but the agent ignored it"
            }
            Self::StateMachineViolation => {
                "the agent made an illegal state transition or bypassed state checks"
            }
            Self::ManagedBlockCorruption => "the agent overwrote or deleted managed-block tags",
            Self::ToolExecutionHallucination => {
                "the agent claimed to run a command without a corresponding subprocess call"
            }
            Self::ContextRecoveryFailure => {
                "the agent hit an error but did not refresh its orientation context"
            }
            Self::ToolDiscoveryFailure => {
                "the agent ignored the specialized tool and used a blunt strategy"
            }
            Self::DocDriftBlindness => {
                "the agent trusted stale documentation over the tool's structured output"
            }
            Self::ActionFormatViolation => {
                "the agent failed to produce a parseable structured action after one re-ask"
            }
        }
    }
}

impl std::fmt::Display for ErrorTaxonomy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}

// ---------------------------------------------------------------------------
// Envelope assertions
// ---------------------------------------------------------------------------

/// Error raised while asserting against captured tool output.
#[derive(Debug, thiserror::Error)]
pub enum EvalsError {
    /// Captured stdout is not valid JSON.
    #[error("stdout is not valid JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    /// Captured stdout is JSON but has no usable `ok` field.
    #[error("stdout JSON has no boolean `ok` field")]
    NoOkField,
    /// Fixture materialization failed.
    #[error("failed to materialize eval fixture: {0}")]
    Fixture(#[source] crate::fixture::FixtureError),
}

/// The observable outcome of a tool invocation, parsed from its stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeOutcome {
    /// `ok: true`.
    Ok,
    /// `ok: false`, with the error `code` (if present) and the `command`
    /// fields of any emitted hints.
    Error {
        /// `data.code` from the error envelope, when present.
        code: Option<String>,
        /// `command` of each hint entry, in order.
        hint_commands: Vec<String>,
    },
}

/// Parse a tool's captured stdout into an [`EnvelopeOutcome`].
///
/// Lenient by design: only `ok` is required, so the assertion holds across
/// envelope-version drift. Hints are read from the optional `hints` array.
pub fn parse_envelope(stdout: &str) -> Result<EnvelopeOutcome, EvalsError> {
    let value: Value = serde_json::from_str(stdout).map_err(EvalsError::NotJson)?;
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or(EvalsError::NoOkField)?;
    if ok {
        return Ok(EnvelopeOutcome::Ok);
    }
    let code = value
        .pointer("/data/code")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let hint_commands = value
        .get("hints")
        .and_then(Value::as_array)
        .map(|hints| {
            hints
                .iter()
                .filter_map(|h| h.get("command").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(EnvelopeOutcome::Error {
        code,
        hint_commands,
    })
}

// ---------------------------------------------------------------------------
// Scenario spec
// ---------------------------------------------------------------------------

/// One replayed agent turn: a command the agent *claims* to have run and
/// the captured output of that run.
#[derive(Debug, Clone)]
pub struct AgentStep {
    /// Command line as issued by the agent.
    pub command: String,
    /// Captured stdout of the invocation.
    pub stdout: String,
    /// Captured stderr of the invocation.
    pub stderr: String,
    /// Exit code of the invocation.
    pub exit_code: i32,
    /// Whether a subprocess call was actually observed. `false` models an
    /// agent that claims to have run the command without running it
    /// (hallucination).
    pub executed: bool,
}

/// The replayed trajectory plus the materialized fixture, handed to every
/// deterministic check.
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// The replayed agent turns, in order.
    pub steps: Vec<AgentStep>,
    /// Root of the materialized fixture directory (temporary).
    pub fixture_root: PathBuf,
    /// The distractor registry declared on the scenario, populated by
    /// [`Scenario::run`] — checks read it from here, not from the scenario.
    pub distractors: Vec<Distractor>,
}

/// What kind of misleading content a distractor carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistractorKind {
    /// Documentation that contradicts the tool's current behavior (stale
    /// managed block, outdated README section).
    StaleDocs,
    /// A hint pointing away from the envelope's suggested fix.
    ContradictingHint,
}

/// A distractor file: present in the replay environment but not part of
/// the task's happy path. Whether consulting it is a fault is decided by
/// checks (e.g. [`doc_drift_blindness`]), not by its presence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distractor {
    /// Path relative to the fixture root.
    pub path: String,
    /// File content materialized into the replay environment.
    pub content: String,
    /// What kind of misleading content this is.
    pub kind: DistractorKind,
}

/// Outcome of a single deterministic check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// Check held.
    Pass,
    /// Check failed. `taxonomy` classifies agent-fault failures; `None`
    /// means the tool under test misbehaved (no agent fault).
    Fail {
        /// Classification, when the failure is attributable to the agent.
        taxonomy: Option<ErrorTaxonomy>,
        /// Human-readable, agent-actionable reason.
        reason: String,
    },
}

impl CheckOutcome {
    /// Shorthand for a passing check.
    pub fn pass() -> Self {
        Self::Pass
    }

    /// Tool-fault failure (no agent classification).
    pub fn tool_fault(reason: impl Into<String>) -> Self {
        Self::Fail {
            taxonomy: None,
            reason: reason.into(),
        }
    }

    /// Agent-fault failure classified with a taxonomy code.
    pub fn agent_fault(taxonomy: ErrorTaxonomy, reason: impl Into<String>) -> Self {
        Self::Fail {
            taxonomy: Some(taxonomy),
            reason: reason.into(),
        }
    }
}

/// A named deterministic check over a [`ScenarioResult`].
pub struct ScenarioCheck {
    /// Stable check name for reports.
    pub name: String,
    /// The deterministic predicate.
    pub check: Box<dyn Fn(&ScenarioResult) -> CheckOutcome>,
}

/// An evaluation scenario: fixture setup + agent prompt + deterministic
/// checks, replayed against a fake-agent transcript.
pub struct Scenario {
    /// Stable scenario name.
    pub name: String,
    /// The prompt handed to the (real or replayed) agent.
    pub prompt: String,
    /// Files written into the fixture temp dir before replay.
    pub fixture_files: Vec<(String, String)>,
    /// Distractor files: materialized into the replay environment like
    /// fixtures, but registered separately so checks can tell bait from
    /// task material.
    pub distractors: Vec<Distractor>,
    /// Deterministic checks applied to the replay result.
    pub checks: Vec<ScenarioCheck>,
}

/// Report of a scenario replay.
#[derive(Debug)]
pub struct ScenarioReport {
    /// Scenario name.
    pub name: String,
    /// True when every check passed.
    pub passed: bool,
    /// One entry per failed check: (check name, outcome).
    pub failures: Vec<(String, CheckOutcome)>,
    /// The model that replayed the scenario, when attributed — supports
    /// per-model matrix runs without changing `run()`'s signature
    /// (design D3). Omitted from serialized output when absent.
    pub model: Option<String>,
    /// Root of the materialized fixture directory. Owned here so the temp
    /// dir survives until the report is dropped.
    pub fixture_root: PathBuf,
    /// Holds the fixture alive (temp dir removed on drop).
    _fixture: crate::fixture::Fixture,
}

impl ScenarioReport {
    /// Attribute this replay to a model name (builder style). Matrix runs
    /// call this per model after `run()` — `run()`'s signature is unchanged.
    ///
    /// Record the raw model id **verbatim** — including `:free` and any
    /// deployment suffix — because matrix comparisons across time depend on
    /// the unnormalized id (add-evals-guidelines, free-tier attribution).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Serialize the report for matrix comparison: name, passed, failures
/// (with machine-readable taxonomy codes), and `model` only when set.
impl serde::Serialize for ScenarioReport {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let field_count = 3 + usize::from(self.model.is_some());
        let mut s = serializer.serialize_struct("ScenarioReport", field_count)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("passed", &self.passed)?;
        let failures: Vec<SerializedFailure<'_>> = self
            .failures
            .iter()
            .map(|(check, outcome)| SerializedFailure {
                check,
                taxonomy_code: match outcome {
                    CheckOutcome::Fail { taxonomy, .. } => {
                        taxonomy.as_ref().map(ErrorTaxonomy::code)
                    }
                    CheckOutcome::Pass => None,
                },
                reason: match outcome {
                    CheckOutcome::Fail { reason, .. } => reason,
                    CheckOutcome::Pass => "",
                },
            })
            .collect();
        s.serialize_field("failures", &failures)?;
        if let Some(model) = &self.model {
            s.serialize_field("model", model)?;
        }
        s.end()
    }
}

#[derive(serde::Serialize)]
struct SerializedFailure<'a> {
    check: &'a str,
    taxonomy_code: Option<&'static str>,
    reason: &'a str,
}

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
        let mut builder = crate::fixture::Fixture::new();
        for (path, content) in &self.fixture_files {
            builder = builder.with_file(path, content);
        }
        // Distractors share the replay environment with real fixtures.
        for d in &self.distractors {
            builder = builder.with_file(&d.path, &d.content);
        }
        let fixture = builder.build().map_err(EvalsError::Fixture)?;
        let result = ScenarioResult {
            steps: replay,
            fixture_root: fixture.root().to_path_buf(),
            distractors: self.distractors.clone(),
        };
        let failures: Vec<(String, CheckOutcome)> = self
            .checks
            .iter()
            .filter_map(|c| match (c.check)(&result) {
                CheckOutcome::Pass => None,
                fail @ CheckOutcome::Fail { .. } => Some((c.name.clone(), fail)),
            })
            .collect();
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
        let Some(distractor) = result
            .distractors
            .iter()
            .find(|d| d.kind == DistractorKind::StaleDocs)
        else {
            return CheckOutcome::tool_fault(
                "doc_drift_blindness requires a StaleDocs distractor declared via Scenario::distractor_file",
            );
        };
        // Exact match, or bait as the command word prefix (flags/args must
        // not let doc-following escape the check).
        let bait_prefix = format!("{bait} ");
        if let Some(i) = result
            .steps
            .iter()
            .position(|s| s.command == bait || s.command.starts_with(&bait_prefix))
        {
            return CheckOutcome::agent_fault(
                ErrorTaxonomy::DocDriftBlindness,
                format!(
                    "step {i} ran `{bait}`, consistent with the stale docs at `{}` instead of the tool's envelope",
                    distractor.path
                ),
            );
        }
        match result
            .steps
            .iter()
            .position(|s| s.executed && parse_envelope(&s.stdout).is_ok())
        {
            Some(_) => CheckOutcome::pass(),
            None => CheckOutcome::tool_fault(
                "no executed step received a parseable envelope; nothing to evaluate",
            ),
        }
    }
}

#[cfg(test)]
#[path = "evals/tests.rs"]
mod tests;
