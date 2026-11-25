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
//!
//! The protocol/attribution, report-contract, and fault-routing guards live
//! in `tests/evals_guidelines_protocol.rs`.

mod common;

use common::{norm_doc, norm_markdown, read};

// -- Tier ladder -----------------------------------------------------------

/// Requirement: Tier ladder — scenarios: tier claims are bounded,
/// tier cadences, contrived failures bound prevalence claims.
#[test]
fn tier_ladder_normative_content_is_published() {
    let ladder = norm_doc("docs/src/how-to/evals.md");
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
    let ci = norm_doc("docs/src/how-to/evals-ci.md");
    assert!(
        ci.contains("every push") && ci.contains("nightly"),
        "tier cadences missing from evals-ci.md"
    );
}

// -- Battery maintenance -----------------------------------------------------

/// Requirement: Battery maintenance — never-failing checks are reviewed.
#[test]
fn battery_maintenance_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
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
    // The deployed spec is the other normative home — the never-failing
    // rule must be stated there too.
    let spec = norm_markdown(&read("openspec/specs/evals-guidelines/spec.md"));
    assert!(
        spec.contains("permanently green"),
        "permanently-green rule missing from the deployed spec"
    );
}

// -- Process-boundary scoring -------------------------------------------------

/// Requirement: Process-boundary scoring — envelope is the assertion
/// target; hallucination remains detectable.
#[test]
fn process_boundary_scoring_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
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
    let modules = read("docs/src/reference/modules.md");
    assert!(modules.contains("ErrorTaxonomy"));
}

// -- Fixture provisioning / Sandbox confinement -------------------------------

/// Requirements: Fixture provisioning (tool binary provisioned before the
/// turn) and Sandbox confinement (isolated execution, network denied by
/// default, runaway command terminated).
#[test]
fn provisioning_and_sandbox_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
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
    // Confinement requirements live under a dedicated sandbox heading in
    // the raw (non-normalized) page.
    assert!(
        read("docs/src/how-to/evals.md").contains("Sandbox"),
        "sandbox section heading missing from evals.md"
    );
}

// -- Distractor registration / One fault per check / Empty-trajectory guard ---

/// Requirements: Distractor registration (distractors are not task
/// material), One fault per check, Empty-trajectory guard (passive models
/// cannot pass by doing nothing), Scenario provenance.
#[test]
fn check_discipline_normative_content_is_published() {
    let howto = norm_doc("docs/src/how-to/evals.md");
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
    // The deployed spec carries the "One fault per check" requirement;
    // its normative wording there is "exactly one agent-fault taxonomy code".
    let spec = norm_markdown(&read("openspec/specs/evals-guidelines/spec.md"));
    assert!(
        spec.contains("exactly one agent-fault taxonomy code"),
        "one-fault-per-check rule missing from the deployed spec"
    );
}
