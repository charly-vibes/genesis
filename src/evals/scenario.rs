//! Scenario/check data model and report serialization.

use std::path::PathBuf;

use super::taxonomy::ErrorTaxonomy;

/// One replayed agent turn: a command the agent *claims* to have run and
/// the captured output of that run.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub(super) _fixture: crate::fixture::Fixture,
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
        let failures = serialized_failures(&self.failures);
        let model_field = self.model.as_ref();
        let field_count = report_field_count(model_field.is_some());
        let mut s = serializer.serialize_struct("ScenarioReport", field_count)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("passed", &self.passed)?;
        s.serialize_field("failures", &failures)?;
        if let Some(model) = model_field {
            s.serialize_field("model", model)?;
        }
        s.end()
    }
}

/// Struct field count: the base three plus `model` only when set.
fn report_field_count(model_is_some: bool) -> usize {
    3 + usize::from(model_is_some)
}

/// Failures with machine-readable taxonomy codes for matrix comparison.
fn serialized_failures(failures: &[(String, CheckOutcome)]) -> Vec<SerializedFailure<'_>> {
    failures
        .iter()
        .map(|(check, outcome)| SerializedFailure {
            check,
            taxonomy_code: match outcome {
                CheckOutcome::Fail { taxonomy, .. } => taxonomy.as_ref().map(ErrorTaxonomy::code),
                CheckOutcome::Pass => None,
            },
            reason: match outcome {
                CheckOutcome::Fail { reason, .. } => reason,
                CheckOutcome::Pass => "",
            },
        })
        .collect()
}

#[derive(serde::Serialize)]
struct SerializedFailure<'a> {
    check: &'a str,
    taxonomy_code: Option<&'static str>,
    reason: &'a str,
}
