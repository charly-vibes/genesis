//! Tests for eval distractors + doc-drift blindness check
//! (add-aix-eval-loop §3, design D3).

use genesis::evals::{
    AgentStep, CheckOutcome, DistractorKind, ErrorTaxonomy, Scenario, doc_drift_blindness,
};

fn step(command: &str, stdout: &str, exit_code: i32) -> AgentStep {
    AgentStep {
        command: command.into(),
        stdout: stdout.into(),
        stderr: String::new(),
        exit_code,
        executed: true,
    }
}

fn ok_envelope_stdout() -> String {
    serde_json::json!({"ok": true}).to_string()
}

/// Step that invokes the tool and receives an ok envelope — the
/// envelope-trusting action.
fn envelope_trusting_step(command: &str) -> AgentStep {
    step(command, &ok_envelope_stdout(), 0)
}

// ── Distractor fixtures (tasks 3.1–3.2) ───────────────────────────────

#[test]
fn distractor_lands_in_replay_fixture_env() {
    let scenario = Scenario::new("with-distractor", "prompt")
        .fixture_file("AGENTS.md", "real fixture content\n")
        .distractor_file(
            "docs/cli.md",
            "## Commands\n\n- `my-tool configure` — the way to init\n",
            DistractorKind::StaleDocs,
        );
    let report = scenario
        .run(vec![envelope_trusting_step("my-tool init")])
        .expect("fixture");

    let distractor_path = report.fixture_root.join("docs/cli.md");
    assert!(
        distractor_path.exists(),
        "distractor file must be materialized in the replay environment"
    );
    assert!(report.passed);
}

#[test]
fn distractor_of_every_kind_materializes() {
    for kind in [DistractorKind::StaleDocs, DistractorKind::ContradictingHint] {
        let scenario =
            Scenario::new("kinds", "prompt").distractor_file("hints.txt", "wrong hint\n", kind);
        let report = scenario.run(vec![]).expect("fixture");
        assert!(
            report.fixture_root.join("hints.txt").exists(),
            "kind {kind:?}"
        );
    }
}

#[test]
fn distractor_ignoring_run_passes() {
    // Agent ignores the distractor entirely and satisfies the checks —
    // the distractor must not cause a fault.
    let scenario = Scenario::new("ignores-distractor", "prompt")
        .distractor_file("docs/cli.md", "stale docs\n", DistractorKind::StaleDocs)
        .check("trusts-envelope", doc_drift_blindness("my-tool configure"));
    let replay = vec![envelope_trusting_step("my-tool init")];
    let report = scenario.run(replay).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

// ── Doc-drift blindness check (tasks 3.3–3.4) ─────────────────────────

#[test]
fn doc_drift_blindness_passes_on_envelope_trusting_steps() {
    let scenario = Scenario::new("trusts-envelope", "prompt")
        .distractor_file(
            "docs/cli.md",
            "- `my-tool configure` — stale instruction\n",
            DistractorKind::StaleDocs,
        )
        .check("doc-drift", doc_drift_blindness("my-tool configure"));
    let replay = vec![
        envelope_trusting_step("my-tool --help"),
        envelope_trusting_step("my-tool init"),
    ];
    let report = scenario.run(replay).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

/// Doc-following with flags must not escape the bait check.
#[test]
fn doc_drift_blindness_catches_bait_with_arguments() {
    let scenario = Scenario::new("bait-with-flags", "prompt")
        .distractor_file(
            "docs/cli.md",
            "- `my-tool configure` — stale\n",
            DistractorKind::StaleDocs,
        )
        .check("doc-drift", doc_drift_blindness("my-tool configure"));
    let replay = vec![step("my-tool configure --force", &ok_envelope_stdout(), 0)];
    let report = scenario.run(replay).expect("fixture");
    assert!(
        !report.passed,
        "doc-following with flags is still doc-following"
    );
}

#[test]
fn doc_drift_blindness_fails_doc_following_steps() {
    let scenario = Scenario::new("follows-docs", "prompt")
        .distractor_file(
            "docs/cli.md",
            "- `my-tool configure` — stale instruction\n",
            DistractorKind::StaleDocs,
        )
        .check("doc-drift", doc_drift_blindness("my-tool configure"));
    // Agent acts consistently with the stale doc, not the envelope.
    let replay = vec![
        envelope_trusting_step("my-tool --help"),
        step("my-tool configure", &ok_envelope_stdout(), 0),
    ];
    let report = scenario.run(replay).expect("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy, reason } => {
            assert_eq!(*taxonomy, Some(ErrorTaxonomy::DocDriftBlindness));
            assert!(
                reason.contains("docs/cli.md"),
                "reason must identify the distractor path; got: {reason}"
            );
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn doc_drift_blindness_without_distractor_is_tool_fault() {
    let scenario = Scenario::new("no-distractor", "prompt")
        .check("doc-drift", doc_drift_blindness("my-tool configure"));
    let replay = vec![step("my-tool configure", &ok_envelope_stdout(), 0)];
    let report = scenario.run(replay).expect("fixture");
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy, reason } => {
            assert_eq!(*taxonomy, None, "misconfigured check is a tool fault");
            assert!(reason.contains("StaleDocs"), "got: {reason}");
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

// ── Model attribution (task 3.5) ──────────────────────────────────────

#[test]
fn report_carries_model_when_provided() {
    let scenario =
        Scenario::new("attributed", "prompt").check("always-passes", |_| CheckOutcome::pass());
    let report = scenario
        .run(vec![])
        .expect("fixture")
        .with_model("test-model/1.0");
    let json = serde_json::to_string(&report).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["model"], "test-model/1.0");
}

#[test]
fn report_without_attribution_stays_valid() {
    let scenario =
        Scenario::new("unattributed", "prompt").check("always-passes", |_| CheckOutcome::pass());
    let report = scenario.run(vec![]).expect("fixture");
    let json = serde_json::to_string(&report).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        parsed.get("model").is_none(),
        "model key must be omitted: {json}"
    );
}

// ── End-to-end: stale managed block vs live envelope (task 3.6) ───────

/// The research's doc-drift scenario (corpus B171/B128): a stale managed
/// block in AGENTS.md documents `my-tool configure` as the init command,
/// while the live `--help` envelope says `my-tool init`. The agent must
/// follow the envelope.
#[test]
fn stale_managed_block_vs_live_help_envelope() {
    const STALE_BLOCK: &str =
        "<!-- my-tool:START -->\nRun `my-tool configure` to initialize.\n<!-- my-tool:END -->\n";
    const LIVE_HELP: &str = r#"{"ok": true, "data": {"commands": ["my-tool init"]}}"#;

    let scenario = Scenario::new("doc-drift-e2e", "Initialize my-tool")
        .distractor_file("AGENTS.md", STALE_BLOCK, DistractorKind::StaleDocs)
        .check("doc-drift", doc_drift_blindness("my-tool configure"));

    // Envelope-trusting agent: reads --help, runs the documented-in-envelope
    // command.
    let trusting = vec![
        step("my-tool --help", LIVE_HELP, 0),
        envelope_trusting_step("my-tool init"),
    ];
    let report = scenario.run(trusting).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);

    // Doc-following agent: runs what the stale block says.
    let doc_following = vec![
        step("my-tool --help", LIVE_HELP, 0),
        envelope_trusting_step("my-tool configure"),
    ];
    let report = scenario.run(doc_following).expect("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    assert!(matches!(
        outcome,
        CheckOutcome::Fail {
            taxonomy: Some(ErrorTaxonomy::DocDriftBlindness),
            ..
        }
    ));
}
