//! Tests for feedback → Scenario conversion (add-aix-eval-loop §4, design D4).

use genesis::evals::{AgentStep, Scenario};
use genesis::feedback::context::ContextBundle;
use genesis::feedback::scratch::{ErrorRecord, write_error_scratch};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn bundle() -> ContextBundle {
    ContextBundle {
        tool_name: "my-tool".into(),
        tool_version: "1.0.0".into(),
        command: Some("my-tool sync".into()),
        exit_code: Some(2),
        suggestion_footer: Some("state file corrupt\n  → Run: my-tool repair".into()),
        os_arch: "linux/x86_64".into(),
        shell: None,
        gh_version: None,
        git_remote: None,
        git_branch: None,
        git_dirty: None,
        repo_state: vec![],
        repro_hash: 0x1234,
    }
}

fn recorded_step() -> AgentStep {
    AgentStep {
        command: "my-tool sync".into(),
        stdout: serde_json::json!({
            "ok": false,
            "data": {"code": "E_SYNC", "message": "sync failed"},
            "hints": [{"command": "my-tool repair", "description": "repair state"}]
        })
        .to_string(),
        stderr: String::new(),
        exit_code: 2,
        executed: true,
    }
}

// ── Bundle-only conversion (task 4.1) ─────────────────────────────────

#[test]
fn bundle_only_conversion_yields_prompt_only_scenario() {
    let scenario = Scenario::from_feedback_context(&bundle(), vec![]);
    assert!(
        scenario.fixture_files.is_empty(),
        "bundle-only conversion must not invent fixtures"
    );
    assert!(
        scenario.name.contains("1234"),
        "name should carry the repro hash for traceability; got {}",
        scenario.name
    );
    let report = scenario.run(vec![recorded_step()]).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

#[test]
fn converted_scenario_expects_footer_hint_in_error_envelope() {
    let scenario = Scenario::from_feedback_context(&bundle(), vec![]);
    // The recorded envelope lacks the footer's suggested command.
    let hintless = AgentStep {
        stdout: serde_json::json!({
            "ok": false,
            "data": {"code": "E_SYNC", "message": "sync failed"}
        })
        .to_string(),
        ..recorded_step()
    };
    let report = scenario.run(vec![hintless]).expect("fixture");
    assert!(!report.passed, "missing footer hint must fail the check");
}

#[test]
fn converted_scenario_accepts_bundle_without_command() {
    let mut b = bundle();
    b.command = None;
    b.exit_code = None;
    let scenario = Scenario::from_feedback_context(&b, vec![]);
    let report = scenario.run(vec![recorded_step()]).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

// ── Caller-supplied fixtures (task 4.2) ───────────────────────────────

#[test]
fn caller_supplied_fixtures_become_scenario_fixtures() {
    let fixtures = vec![
        ("config.toml".to_string(), "key = 1\n".to_string()),
        ("state/db.json".to_string(), "{}".to_string()),
    ];
    let scenario = Scenario::from_feedback_context(&bundle(), fixtures.clone());
    assert_eq!(
        scenario.fixture_files, fixtures,
        "fixtures pass through verbatim"
    );
    let report = scenario.run(vec![recorded_step()]).expect("fixture");
    assert!(report.passed);
    assert!(report.fixture_root.join("config.toml").exists());
    assert!(report.fixture_root.join("state/db.json").exists());
}

// ── Last-error path (task 4.3) ────────────────────────────────────────

#[test]
fn from_last_error_converts_scratch_record() {
    let _guard = ENV_LOCK.lock().unwrap();
    let cache = std::env::temp_dir().join(format!("genesis-m3p-test-{}", std::process::id()));
    std::fs::create_dir_all(&cache).unwrap();
    let prev = std::env::var("XDG_CACHE_HOME").ok();
    unsafe { std::env::set_var("XDG_CACHE_HOME", &cache) };

    let record = ErrorRecord {
        ts: "2026-09-17T12:00:00Z".into(),
        argv: vec!["my-tool".into(), "sync".into()],
        exit: 2,
        footer: Some("→ Run: my-tool repair".into()),
        kind: "sync-failure".into(),
    };
    write_error_scratch("m3p-tool", &record).expect("scratch write");

    // Happy path: record exists, scenario reflects argv/exit/footer.
    let scenario = genesis::feedback::from_last_error("m3p-tool", vec![]).expect("record exists");
    let report = scenario.run(vec![recorded_step()]).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);

    // Missing record: typed error, not a panic.
    let result = genesis::feedback::from_last_error("m3p-never-written", vec![]);
    assert!(matches!(
        result,
        Err(genesis::feedback::ConversionError::NoScratchRecord(_))
    ));

    // Restore env.
    match prev {
        Some(v) => unsafe { std::env::set_var("XDG_CACHE_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CACHE_HOME") },
    }
    std::fs::remove_dir_all(&cache).ok();
}

// ── No-LLM / no-subprocess guarantee (task 4.4) ───────────────────────

#[test]
fn conversion_and_replay_run_in_process() {
    // The whole pipeline — conversion, fixture materialization, replay,
    // checks — is pure in-process computation: no LLM call, no subprocess
    // runner. Executed-ness of recorded steps is metadata, never re-run.
    let mut step = recorded_step();
    step.executed = false;
    let scenario = Scenario::from_feedback_context(&bundle(), vec![]);
    let report = scenario.run(vec![step]).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
    assert!(report.fixture_root.exists());
}
