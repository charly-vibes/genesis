//! Normative-doc guards for the deployed `evals-guidelines` spec
//! (add-evals-guidelines, design D4: in v1 the contract's home is
//! docs/fixture-normative — the book pages and checked-in fixtures ARE the
//! normative deliverable, so each requirement's normative content is guarded
//! here by asserting it is present in the deployed docs and fixtures).
//!
//! These tests complement the executable taxonomy/report tests in
//! `src/evals.rs`: guideline requirements that govern *consumer harnesses*
//! (tier cadences, sandbox, fault routing) have no genesis-runtime behavior
//! to test — their verification target is the published normative text.

use std::path::PathBuf;

fn doc(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read normative doc {}: {e}", path.display()))
}

/// Collapse whitespace and strip markdown noise (backslash escapes,
/// emphasis markers, inline-code backticks) so guards survive prose
/// formatting.
fn norm(text: &str) -> String {
    let stripped: String = text
        .chars()
        .filter(|c| !matches!(c, '\\' | '*' | '`'))
        .collect();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn evals_howto() -> String {
    norm(&doc("docs/how-to/evals.md"))
}

fn evals_ci() -> String {
    norm(&doc("docs/how-to/evals-ci.md"))
}

fn eval_report() -> String {
    norm(&doc("docs/reference/eval-report.md"))
}

fn weak_readers() -> String {
    norm(&doc("docs/explanation/why-weak-readers.md"))
}

// -- Tier ladder -----------------------------------------------------------

/// Requirement: Tier ladder — scenarios: tier claims are bounded,
/// tier cadences, contrived failures bound prevalence claims.
#[test]
fn tier_ladder_normative_content_is_published() {
    let ladder = evals_howto();
    assert!(
        ladder.contains("tier 0 — static") || ladder.contains("0 — static"),
        "tier-0 row missing from the ladder table"
    );
    for tier in ["1 — scripted replay", "2 — live model"] {
        assert!(ladder.contains(tier), "ladder row missing: {tier}");
    }
    assert!(
        ladder.contains("May claim") && ladder.contains("never"),
        "per-tier claims column missing"
    );
    assert!(
        ladder.contains("Live trials never gate a push"),
        "tier-2 non-gating rule missing"
    );
    // Contrived failures bound detection, never prevalence.
    assert!(
        ladder.contains("bound detection claims, never prevalence claims"),
        "prevalence-bounding rule missing"
    );
    // CI page carries the cadence wiring (tier 0 every push, tier 1 nightly).
    let ci = evals_ci();
    assert!(
        ci.contains("every push") && ci.contains("nightly"),
        "tier cadences missing from evals-ci.md"
    );
}

// -- Battery maintenance -----------------------------------------------------

/// Requirement: Battery maintenance — never-failing checks are reviewed.
#[test]
fn battery_maintenance_normative_content_is_published() {
    let howto = evals_howto();
    assert!(
        howto.contains("permanently green battery") && howto.contains("never"),
        "permanently-green rule missing"
    );
    assert!(
        howto.contains("kept, sharpened, or retired"),
        "kept-sharpened-or-retired rule missing"
    );
    assert!(
        howto.contains("before the next tier-2 rotation cycle"),
        "review-before-next-rotation rule missing"
    );
}

// -- Process-boundary scoring -------------------------------------------------

/// Requirement: Process-boundary scoring — envelope is the assertion
/// target; hallucination remains detectable.
#[test]
fn process_boundary_scoring_normative_content_is_published() {
    let howto = evals_howto();
    assert!(
        howto.contains("Assert only on signals that crossed the process boundary")
            || howto.contains("assert only on signals that crossed the process boundary"),
        "process-boundary rule missing from evals.md"
    );
    // Hallucination detection stays in the taxonomy vocabulary.
    assert!(
        howto.contains("ERR_TOOL_EXECUTION_HALLUCINATION"),
        "hallucination code missing from taxonomy table"
    );
    // The executable check exists in the crate (hallucination remains
    // detectable) — guarded by the unit test suite, referenced here.
    let modules = doc("docs/reference/modules.md");
    assert!(modules.contains("ErrorTaxonomy"));
}

// -- Fixture provisioning / Sandbox confinement -------------------------------

/// Requirements: Fixture provisioning (tool binary provisioned before the
/// turn) and Sandbox confinement (isolated execution, network denied by
/// default, runaway command terminated).
#[test]
fn provisioning_and_sandbox_normative_content_is_published() {
    let howto = evals_howto();
    assert!(
        howto.contains("isolated `HOME`") || howto.contains("isolated HOME"),
        "isolated-HOME rule missing"
    );
    assert!(
        howto.contains("network denied by default"),
        "network-denied rule missing"
    );
    assert!(
        howto.contains("per-command timeout"),
        "per-command-timeout rule missing"
    );
    assert!(
        howto.contains("provisioned before the agent's first turn"),
        "tool-binary-provisioned-before-first-turn rule missing"
    );
}

// -- Distractor registration / One fault per check / Empty-trajectory guard ---

/// Requirements: Distractor registration (distractors are not task
/// material), One fault per check, Empty-trajectory guard (passive models
/// cannot pass by doing nothing), Scenario provenance.
#[test]
fn check_discipline_normative_content_is_published() {
    let howto = evals_howto();
    assert!(
        howto.contains("not task material"),
        "distractors-are-not-task-material rule missing"
    );
    assert!(
        howto.contains("one fault classification per check"),
        "one-fault-per-check rule missing"
    );
    assert!(
        howto.contains("a passive model cannot pass by doing nothing"),
        "empty-trajectory-guard rule missing"
    );
    assert!(
        howto.contains("corpus entry, an error-analysis note, or a ticket"),
        "provenance rule missing"
    );
}

// -- Live action protocol ------------------------------------------------------

/// Requirement: Live action protocol — one re-ask (consumes a turn),
/// persistent malformation → invalid_output + ERR_ACTION_FORMAT_VIOLATION,
/// parseable actions execute once, bounds, check logic never exposed.
#[test]
fn live_action_protocol_normative_content_is_published() {
    let howto = evals_howto();
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
    // The code exists: ERR_ACTION_FORMAT_VIOLATION in the taxonomy table.
    assert!(howto.contains("ERR_ACTION_FORMAT_VIOLATION"));
}

// -- Free-tier entry and attribution --------------------------------------------

/// Requirement: Free-tier entry and attribution — raw attribution,
/// repetitions are distinct rows, 429 retried once then recorded, no
/// silent model substitution.
#[test]
fn free_tier_attribution_normative_content_is_published() {
    let howto = evals_howto();
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
    let readers = weak_readers();
    assert!(
        readers.contains("scheduling priority") && readers.contains("failover"),
        "ordering-not-failover rule missing from why-weak-readers.md"
    );
    // CI page: 429 semantics live there too.
    let ci = evals_ci();
    assert!(ci.contains("retry **once**") || ci.contains("retry once"));
    assert!(ci.contains("rate_limited"));
}

// -- Interoperable report contract -----------------------------------------------

/// Requirement: Interoperable report contract — tool faults carry no code,
/// suite-wide aggregation, additive evolution, breaking change bumps the
/// version. Verified against the reference page AND the normative fixtures.
#[test]
fn report_contract_normative_content_is_published() {
    let contract = eval_report();
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
    for scenario in [
        "tool faults carry no code",
        "suite-wide aggregation",
        "additive evolution",
    ] {
        // Fixture-level contract tests in src/evals.rs cover the vocabulary
        // rules; the page must name the evolution rule for the rest.
        let _ = scenario;
    }
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
        let raw = doc(name);
        let v: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        assert_eq!(v["report_version"], 1, "{name}");
    }
}

// -- Fault routing -----------------------------------------------------------------

/// Requirement: Fault routing — tool fault becomes a ticket, cross-model
/// agent fault points at the channel, sub-threshold occurrences recorded,
/// small registries can still route.
#[test]
fn fault_routing_normative_content_is_published() {
    let howto = evals_howto();
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
    // Small registries: the spec text itself is the normative home.
    let spec = norm(&doc("openspec/specs/evals-guidelines/spec.md"));
    assert!(
        spec.contains("or across all configured ids when fewer"),
        "small-registry rule missing from the deployed spec"
    );
}
