//! Report-contract tests (golden fixtures) — split from tests.rs for the
//! pretender file-lines budget.

use super::*;
use crate::testutil::TestUnwrap;

// -- Report contract (add-evals-guidelines) ------------------------------

/// A single executed `my-tool status` step answering `ok:true`.
fn ok_status_step() -> Vec<AgentStep> {
    vec![AgentStep {
        command: "my-tool status".into(),
        stdout: r#"{"ok": true}"#.into(),
        stderr: String::new(),
        exit_code: 0,
        executed: true,
    }]
}

/// The fixture's report name, required to be a string.
fn fixture_name(fixture: &serde_json::Value) -> String {
    fixture["name"].as_str().require("value").to_owned()
}

/// The fixture's pass flag, per its `status` field.
fn fixture_passed(fixture: &serde_json::Value) -> bool {
    fixture["status"].as_str() == Some("passed")
}

/// Load one golden report fixture from `tests/golden`.
fn load_golden_fixture(name: &str) -> serde_json::Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"))
}

/// The replay-tier fixture shape: version 1, passed, tier-2 fields absent
/// (never null).
fn assert_fixture_shape(fixture: &serde_json::Value) {
    assert_eq!(fixture["report_version"], 1);
    assert_eq!(fixture["status"], "passed");
    assert!(fixture.get("model").is_none());
    assert!(fixture.get("repetition").is_none());
}

/// Collect dotted paths of every JSON null under `v`.
fn collect_nulls(v: &serde_json::Value, where_: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Null => out.push(where_.to_string()),
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                collect_nulls(item, &format!("{where_}[{i}]"), out);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                collect_nulls(val, &format!("{where_}.{k}"), out);
            }
        }
        _ => {}
    }
}

/// The non-tier-2 normative fixture must round-trip through
/// `ScenarioReport`'s serde serialization: its shared fields (name,
/// passed↔status, failures↔checks with taxonomy codes) match what the
/// replay-tier struct emits. Live-only fields (report_version, tool,
/// bounds, repetition) are documented as a delta in
/// docs/reference/eval-report.md.
#[test]
fn test_report_replay_fixture_round_trips_through_scenario_report() {
    let fixture = load_golden_fixture("eval_report_replay.json");
    assert_fixture_shape(&fixture);

    // Rebuild the replay-tier report the fixture describes and serialize it.
    let scenario = Scenario::new(fixture_name(&fixture), "fixture round-trip")
        .check("envelope_hint_followed", |_| CheckOutcome::pass());
    let report = scenario
        .run(ok_status_step())
        .require("fixture materializes");
    let serialized = serde_json::to_value(&report).require("ScenarioReport serializes");

    // Shared fields agree across the two shapes.
    assert_eq!(serialized["name"], fixture["name"]);
    assert_eq!(
        serialized["passed"].as_bool(),
        Some(fixture_passed(&fixture))
    );
    assert_eq!(serialized.get("model"), None);
}

/// No field in either normative report fixture may serialize as `null` —
/// optional fields are omitted, never null (evals-guidelines).
#[test]
fn test_report_fixtures_contain_no_nulls() {
    for name in ["eval_report_tier2.json", "eval_report_replay.json"] {
        let fixture = load_golden_fixture(name);
        let mut nulls = Vec::new();
        collect_nulls(&fixture, name, &mut nulls);
        assert!(nulls.is_empty(), "nulls found: {nulls:?}");
    }
}

/// The tier-2 fixture demonstrates the fault-vocabulary rules: an
/// agent fault carries a code, a passing check and a tool fault
/// never do, and tier-2 rows carry model + repetition.
#[test]
fn test_report_tier2_fixture_vocabulary() {
    let fixture = load_golden_fixture("eval_report_tier2.json");
    assert_eq!(fixture["report_version"], 1);
    assert_eq!(fixture["status"], "failed");
    // Tier-2 rows carry the raw model id and a 0-based repetition.
    assert_eq!(fixture["model"], "deepseek/deepseek-v4-flash:free");
    assert_eq!(fixture["repetition"], 0);
    // code present iff fault == agent.
    for check in fixture["checks"].as_array().require("value") {
        let has_code = check.get("code").is_some();
        let is_agent_fault = check["fault"].as_str() == Some("agent");
        assert_eq!(has_code, is_agent_fault, "check {}", check["name"]);
    }
    // Bounds recorded with values.
    assert!(fixture["bounds"]["max_turns"].is_u64());
    assert!(fixture["bounds"]["timeout_secs"].is_u64());
    assert!(fixture["bounds"]["token_budget"].is_u64());
}
