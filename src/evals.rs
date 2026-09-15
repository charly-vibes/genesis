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
    /// Root of the materialized fixture directory. Owned here so the temp
    /// dir survives until the report is dropped.
    pub fixture_root: PathBuf,
    /// Holds the fixture alive (temp dir removed on drop).
    _fixture: crate::fixture::Fixture,
}

impl Scenario {
    /// Start building a scenario.
    pub fn new(name: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            fixture_files: Vec::new(),
            checks: Vec::new(),
        }
    }

    /// Add a file to the fixture (path relative to the fixture root).
    pub fn fixture_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.fixture_files.push((path.into(), content.into()));
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
        let fixture = builder.build().map_err(EvalsError::Fixture)?;
        let result = ScenarioResult {
            steps: replay,
            fixture_root: fixture.root().to_path_buf(),
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

/// Assert the agent's step at `recovery_index` actually ran the suggested
/// fix command. Agent fault (`ERR_ENVELOPE_HINT_BLINDNESS`) if a
/// different command was issued.
pub fn agent_followed_hint(
    recovery_index: usize,
    suggested_command: &str,
) -> impl Fn(&ScenarioResult) -> CheckOutcome + '_ {
    let suggested = suggested_command.to_owned();
    move |result| match result.steps.get(recovery_index) {
        Some(s) if s.command == suggested => CheckOutcome::pass(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- Taxonomy -----------------------------------------------------------

    #[test]
    fn test_taxonomy_codes_round_trip() {
        let all = [
            ErrorTaxonomy::EnvelopeHintBlindness,
            ErrorTaxonomy::StateMachineViolation,
            ErrorTaxonomy::ManagedBlockCorruption,
            ErrorTaxonomy::ToolExecutionHallucination,
            ErrorTaxonomy::ContextRecoveryFailure,
            ErrorTaxonomy::ToolDiscoveryFailure,
        ];
        for t in all {
            assert_eq!(ErrorTaxonomy::from_code(t.code()), Some(t));
        }
    }

    #[test]
    fn test_taxonomy_unknown_code_is_none() {
        assert_eq!(ErrorTaxonomy::from_code("ERR_NOPE"), None);
        assert_eq!(ErrorTaxonomy::from_code(""), None);
    }

    #[test]
    fn test_taxonomy_display_contains_code() {
        let text = ErrorTaxonomy::ManagedBlockCorruption.to_string();
        assert!(text.starts_with("ERR_MANAGED_BLOCK_CORRUPTION"));
    }

    // -- Envelope parsing ----------------------------------------------------

    #[test]
    fn test_parse_envelope_ok() {
        let out = parse_envelope(r#"{"ok": true}"#).unwrap();
        assert_eq!(out, EnvelopeOutcome::Ok);
    }

    #[test]
    fn test_parse_envelope_error_with_hints() {
        let out = parse_envelope(
            r#"{"ok": false, "data": {"code": "E_CHECK"},
                "hints": [{"command": "my-tool fix", "description": "run fix"}]}"#,
        )
        .unwrap();
        match out {
            EnvelopeOutcome::Error {
                code,
                hint_commands,
            } => {
                assert_eq!(code.as_deref(), Some("E_CHECK"));
                assert_eq!(hint_commands, vec!["my-tool fix".to_string()]);
            }
            other => panic!("expected error envelope, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_envelope_error_without_hints() {
        let out = parse_envelope(r#"{"ok": false, "data": {}}"#).unwrap();
        match out {
            EnvelopeOutcome::Error {
                code,
                hint_commands,
            } => {
                assert_eq!(code, None);
                assert!(hint_commands.is_empty());
            }
            other => panic!("expected error envelope, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_envelope_rejects_non_json() {
        let err = parse_envelope("panic: boom").unwrap_err();
        assert!(matches!(err, EvalsError::NotJson(_)));
    }

    #[test]
    fn test_parse_envelope_rejects_missing_ok() {
        let err = parse_envelope(r#"{"status": "fine"}"#).unwrap_err();
        assert!(matches!(err, EvalsError::NoOkField));
    }

    // -- End-to-end scenario --------------------------------------------------

    const MANAGED_BLOCK: &str = "<!-- my-tool:START -->\nmanaged\n<!-- my-tool:END -->\n";

    fn error_envelope_stdout(suggested: &str) -> String {
        serde_json::json!({
            "ok": false,
            "data": {"code": "E_STATE", "message": "bad state"},
            "hints": [{"command": suggested, "description": "suggested fix"}]
        })
        .to_string()
    }

    fn ok_envelope_stdout() -> String {
        serde_json::json!({"ok": true}).to_string()
    }

    /// Worked end-to-end scenario: fixture + fake-agent replay +
    /// deterministic checks + taxonomy classification. No live LLM, no
    /// subprocess runner — the transcript is replayed in memory.
    #[test]
    fn scenario_error_hint_followed_end_to_end() {
        let scenario = Scenario::new("hint-followed", "Run my-tool check and fix what breaks")
            .fixture_file("AGENTS.md", MANAGED_BLOCK)
            .check(
                "step0-error-envelope-with-hint",
                error_envelope_with_hint(0, "my-tool fix"),
            )
            .check("step0-nonzero-exit", |r| {
                // Portable assertion: nonzero + parseable envelope (holds
                // before/after genesis-u40 exit-code refinement).
                if r.steps[0].exit_code != 0 {
                    CheckOutcome::pass()
                } else {
                    CheckOutcome::tool_fault("step 0 exited 0")
                }
            })
            .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"))
            .check("recovery-ok-envelope", ok_envelope(1))
            .check("agent-executed-all", agent_executed_all());

        let replay = vec![
            AgentStep {
                command: "my-tool check".into(),
                stdout: error_envelope_stdout("my-tool fix"),
                stderr: String::new(),
                exit_code: 1,
                executed: true,
            },
            AgentStep {
                command: "my-tool fix".into(),
                stdout: ok_envelope_stdout(),
                stderr: String::new(),
                exit_code: 0,
                executed: true,
            },
        ];

        let report = scenario.run(replay).expect("fixture");
        assert!(report.passed, "failures: {:?}", report.failures);
        assert!(report.failures.is_empty());
        // Fixture materialized for checks.
        assert!(report.fixture_root.join("AGENTS.md").exists());
    }

    /// The same scenario against a hint-blind transcript classifies the
    /// failure as ERR_ENVELOPE_HINT_BLINDNESS.
    #[test]
    fn scenario_classifies_hint_blindness() {
        let scenario = Scenario::new("hint-blind", "prompt")
            .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"));
        let replay = vec![
            AgentStep {
                command: "my-tool check".into(),
                stdout: error_envelope_stdout("my-tool fix"),
                stderr: String::new(),
                exit_code: 1,
                executed: true,
            },
            AgentStep {
                command: "rm -rf /".into(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 1,
                executed: true,
            },
        ];
        let report = scenario.run(replay).expect("fixture");
        assert!(!report.passed);
        assert_eq!(report.failures.len(), 1);
        let (_, outcome) = &report.failures[0];
        match outcome {
            CheckOutcome::Fail {
                taxonomy: Some(t), ..
            } => assert_eq!(*t, ErrorTaxonomy::EnvelopeHintBlindness),
            other => panic!("expected agent fault, got {other:?}"),
        }
    }

    /// A step with `executed: false` classifies as
    /// ERR_TOOL_EXECUTION_HALLUCINATION.
    #[test]
    fn scenario_classifies_hallucination() {
        let scenario = Scenario::new("hallucinated", "prompt")
            .check("agent-executed-all", agent_executed_all());
        let replay = vec![AgentStep {
            command: "my-tool check".into(),
            stdout: ok_envelope_stdout(),
            stderr: String::new(),
            exit_code: 0,
            executed: false,
        }];
        let report = scenario.run(replay).expect("fixture");
        assert!(!report.passed);
        let (_, outcome) = &report.failures[0];
        match outcome {
            CheckOutcome::Fail {
                taxonomy: Some(t), ..
            } => assert_eq!(*t, ErrorTaxonomy::ToolExecutionHallucination),
            other => panic!("expected agent fault, got {other:?}"),
        }
    }

    /// A failing recovery envelope is a TOOL fault (fix issued but
    /// ineffective), not ERR_ENVELOPE_HINT_BLINDNESS.
    #[test]
    fn ok_envelope_failing_recovery_is_tool_fault_not_blindness() {
        let scenario = Scenario::new("ineffective-fix", "prompt")
            .check("recovery-ok-envelope", ok_envelope(1));
        let replay = vec![
            AgentStep {
                command: "my-tool check".into(),
                stdout: error_envelope_stdout("my-tool fix"),
                stderr: String::new(),
                exit_code: 1,
                executed: true,
            },
            AgentStep {
                command: "my-tool fix".into(),
                stdout: error_envelope_stdout("my-tool fix"),
                stderr: String::new(),
                exit_code: 1,
                executed: true,
            },
        ];
        let report = scenario.run(replay).expect("fixture");
        assert!(!report.passed);
        let (_, outcome) = &report.failures[0];
        match outcome {
            CheckOutcome::Fail { taxonomy: None, .. } => {}
            other => panic!("expected tool fault, got {other:?}"),
        }
    }

    /// Step-indexed checks against an empty replay report tool faults,
    /// not panics.
    #[test]
    fn empty_replay_yields_tool_faults_for_step_checks() {
        let scenario = Scenario::new("empty", "prompt")
            .check(
                "step0-hint-envelope",
                error_envelope_with_hint(0, "my-tool fix"),
            )
            .check("step0-ok-envelope", ok_envelope(0));
        let report = scenario.run(vec![]).expect("fixture");
        assert!(!report.passed);
        assert_eq!(report.failures.len(), 2);
        for (_, outcome) in &report.failures {
            match outcome {
                CheckOutcome::Fail { taxonomy: None, .. } => {}
                other => panic!("expected tool fault, got {other:?}"),
            }
        }
    }

    /// Tool-fault failures (hintless error envelope) carry no taxonomy.
    #[test]
    fn scenario_tool_fault_has_no_taxonomy() {
        let scenario = Scenario::new("hintless-tool", "prompt").check(
            "step0-error-envelope-with-hint",
            error_envelope_with_hint(0, "my-tool fix"),
        );
        let replay = vec![AgentStep {
            command: "my-tool check".into(),
            stdout: r#"{"ok": false, "data": {}}"#.into(), // error, no hints
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        }];
        let report = scenario.run(replay).expect("fixture");
        assert!(!report.passed);
        let (_, outcome) = &report.failures[0];
        match outcome {
            CheckOutcome::Fail { taxonomy: None, .. } => {}
            other => panic!("expected tool fault, got {other:?}"),
        }
    }
}
