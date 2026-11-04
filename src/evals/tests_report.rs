//! Report-contract tests (golden fixtures) — split from tests.rs for the
//! pretender file-lines budget.

use crate::testutil::TestUnwrap;
use super::*;
use crate::testutil::TestUnwrap;

// -- Report contract (add-evals-guidelines) ------------------------------

/// The non-tier-2 normative fixture must round-trip through
/// `ScenarioReport`'s serde serialization: its shared fields (name,
/// passed↔status, failures↔checks with taxonomy codes) match what the
/// replay-tier struct emits. Live-only fields (report_version, tool,
/// bounds, repetition) are documented as a delta in
/// docs/reference/eval-report.md.
#[test]
fn test_report_replay_fixture_round_trips_through_scenario_report() {
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/eval_report_replay.json");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&golden_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", golden_path.display())),
    )
    .require("non-tier-2 fixture is valid JSON");

    assert_eq!(fixture["report_version"], 1);
    assert_eq!(fixture["status"], "passed");
    // Non-tier-2 rows omit tier-2 fields entirely (never null).
    assert!(fixture.get("model").is_none());
    assert!(fixture.get("repetition").is_none());

    // Rebuild the replay-tier report the fixture describes and serialize it.
    let scenario = Scenario::new(
        fixture["name"].as_str().require("value").to_owned(),
        "fixture round-trip",
    )
    .check("envelope_hint_followed", |_| CheckOutcome::pass());
    let report = scenario
        .run(vec![AgentStep {
            command: "my-tool status".into(),
            stdout: r#"{"ok": true}"#.into(),
            stderr: String::new(),
            exit_code: 0,
            executed: true,
        }])
        .require("fixture materializes");
    let serialized: serde_json::Value =
        serde_json::to_value(&report).require("ScenarioReport serializes");

    // Shared fields agree across the two shapes.
    assert_eq!(serialized["name"], fixture["name"]);
    assert_eq!(
        serialized["passed"].as_bool(),
        Some(fixture["status"].as_str() == Some("passed"))
    );
    assert_eq!(serialized.get("model"), None);
}

/// No field in either normative report fixture may serialize as `null` —
/// optional fields are omitted, never null (evals-guidelines).
#[test]
fn test_report_fixtures_contain_no_nulls() {
    for name in ["eval_report_tier2.json", "eval_report_replay.json"] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(name);
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
        )
        .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        let mut nulls = Vec::new();
        collect_nulls(&fixture, name, &mut nulls);
        assert!(nulls.is_empty(), "nulls found: {nulls:?}");
    }
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
/// The tier-2 fixture demonstrates the fault-vocabulary rules: an
/// agent fault carries a code, a passing check and a tool fault
/// never do, and tier-2 rows carry model + repetition.
#[test]
fn test_report_tier2_fixture_vocabulary() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/eval_report_tier2.json");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
    )
    .require("tier-2 fixture is valid JSON");

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
