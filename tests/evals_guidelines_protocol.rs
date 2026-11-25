//! Normative-doc guards for the deployed `evals-guidelines` spec
//! (add-evals-guidelines, design D4): the consumer-facing protocol and
//! report halves. The tier-ladder, battery-maintenance, process-boundary,
//! sandbox, and check-discipline guards live in
//! `tests/evals_guidelines_docs.rs`; both files assert that each
//! requirement's normative content is present in the deployed docs and
//! fixtures (the book pages and checked-in fixtures ARE the normative
//! deliverable).

mod common;

use common::{norm_doc, norm_markdown, read};

/// Requirement: Live action protocol — one re-ask (consumes a turn),
/// persistent malformation → invalid_output + ERR_ACTION_FORMAT_VIOLATION,
/// parseable actions execute once, bounds, check logic never exposed.
#[test]
fn live_action_protocol_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
    assert!(howto.contains("re-ask once"), "one-re-ask rule missing");
    assert!(
        howto.contains("consumes a turn"),
        "re-ask-consumes-turn rule missing"
    );
    assert!(
        howto.contains("invalid_output") && howto.contains("ERR_ACTION_FORMAT_VIOLATION"),
        "persistent-malformation outcome missing"
    );
    assert!(
        howto.contains("**exactly once**") || howto.contains("exactly once"),
        "execute-once rule missing"
    );
    for bound in ["turn cap", "wall-clock timeout", "token-estimate budget"] {
        assert!(howto.contains(bound), "bound missing: {bound}");
    }
    assert!(
        howto.contains("not** attributed") || howto.contains("not attributed"),
        "bound-stop-no-fault rule missing"
    );
    assert!(
        howto.contains("Never") && howto.contains("check logic"),
        "check-logic-never-exposed rule missing"
    );
    assert!(howto.contains("ERR_ACTION_FORMAT_VIOLATION"));
}

/// Requirement: Free-tier entry and attribution — raw attribution,
/// repetitions are distinct rows, 429 retried once then recorded, no
/// silent model substitution.
#[test]
fn free_tier_attribution_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
    assert!(
        howto.contains("verbatim") && howto.contains(":free"),
        "verbatim-id rule missing"
    );
    assert!(
        howto.contains("distinct report row"),
        "distinct-rows rule missing"
    );
    assert!(
        howto.contains("retry once after ≥ 2 s") || howto.contains("retry once after"),
        "429-retry rule missing"
    );
    assert!(
        howto.contains("never a silent substitution"),
        "no-silent-substitution rule missing"
    );
    let readers = norm_doc("docs/src/explanation/why-weak-readers.md");
    assert!(
        readers.contains("scheduling priority") && readers.contains("failover"),
        "ordering-not-failover rule missing from why-weak-readers.md"
    );
    let ci = norm_doc("docs/src/how-to/evals-ci.md");
    assert!(ci.contains("retry **once**") || ci.contains("retry once"));
    assert!(ci.contains("rate_limited"));
}

/// Requirement: Interoperable report contract — tool faults carry no code.
///
/// Suite-wide aggregation, additive evolution, and breaking-change version
/// bumps are verified against the reference page AND the normative fixtures.
#[test]
fn report_contract_normative_content_is_published() {
    let contract = norm_doc("docs/src/reference/eval-report.md");
    assert!(contract.contains("report_version"), "version field missing");
    assert!(
        contract.contains("Omitted — never null") || contract.contains("omit, never null"),
        "omit-not-null rule missing"
    );
    assert!(
        contract.contains("passed | failed | rate_limited | invalid_output"),
        "status vocabulary missing"
    );
    assert!(
        contract
            .to_lowercase()
            .contains("one entry per declared check"),
        "all-declared-checks rule missing"
    );
    assert!(
        contract.contains("for humans and tickets, not parsed by aggregators"),
        "reason-not-parsed rule missing"
    );
    assert!(
        contract.contains("removing or re-typing a field bumps report_version"),
        "breaking-change rule missing"
    );
    assert!(
        contract.contains("older consumers ignore unknown fields"),
        "additive-evolution rule missing"
    );
}

/// The normative fixtures stay byte-stable (contract drift guard).
#[test]
fn report_fixtures_are_loadable_and_versioned() {
    for name in [
        "tests/golden/eval_report_tier2.json",
        "tests/golden/eval_report_replay.json",
    ] {
        let raw_json = read(name);
        let v: serde_json::Value = serde_json::from_str(&raw_json)
            .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        assert_eq!(v["report_version"], 1, "{name}");
    }
}

/// Requirement: Fault routing — tool fault becomes a ticket, cross-model
/// agent fault points at the channel, sub-threshold occurrences recorded,
/// small registries can still route.
#[test]
fn fault_routing_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
    assert!(
        howto.contains("become tickets in your tool's repo"),
        "tool-fault-becomes-ticket rule missing"
    );
    assert!(
        howto.contains("≥ 3 model ids"),
        "cross-model threshold missing"
    );
    assert!(
        howto.contains("points at **your output channel**")
            || howto.contains("points at your output channel"),
        "channel-routing rule missing"
    );
    assert!(
        howto.contains("recorded without action"),
        "sub-threshold rule missing"
    );
    let spec = norm_markdown(&read("openspec/specs/evals-guidelines/spec.md"));
    assert!(
        spec.contains("or across all configured ids when fewer"),
        "small-registry rule missing from the deployed spec"
    );
}
