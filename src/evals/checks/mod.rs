//! `Scenario::run` replay and the canned check functions (the `canned`
//! submodule, re-exported here).

use super::outcome::EvalsError;
use super::scenario::{
    AgentStep, CheckOutcome, Distractor, DistractorKind, Scenario, ScenarioCheck, ScenarioReport,
    ScenarioResult,
};

mod canned;

pub use canned::{
    agent_executed_all, agent_followed_hint, doc_drift_blindness, error_envelope_with_hint,
    ok_envelope, receipt_records_terminal_outcome,
};

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
