//! Tests for AIX token-cost estimation and budget-bounded generation
//! (add-aix-eval-loop §2, design D2).
//!
//! Degradation-ladder tests derive budgets relative to the full artifact's
//! estimate so they stay valid if fixtures evolve: the ladder's deterministic
//! stages have known relative savings, and the budgets are chosen to land
//! between stages.

use genesis::aix::{
    LlmSection, ModuleEntry, ProjectMeta, estimate_token_cost, generate_llm_txt,
    generate_llm_txt_bounded, generate_llms_txt, generate_llms_txt_bounded,
};
use std::fs;
use std::path::PathBuf;

fn est(s: &str) -> usize {
    estimate_token_cost(s).estimate
}

fn fixture_meta() -> ProjectMeta {
    ProjectMeta::new("my-tool", "One-line tagline.")
        .with_repository("https://github.com/example/my-tool")
        .with_documentation("https://example.com/docs")
}

fn fixture_modules() -> Vec<ModuleEntry> {
    vec![
        ModuleEntry::new(
            "module_a",
            "Handles the primary workload with care. This second sentence is extra detail that padding measures will remove.",
        ),
        ModuleEntry::new(
            "module_b",
            "Auxiliary support routines. Second sentence with more words that only matters when the budget is generous.",
        ),
    ]
}

fn fixture_sections() -> Vec<LlmSection> {
    vec![
        LlmSection::heading(
            "Overview",
            "The overview explains the tool. This second sentence is detail a tight budget drops.",
        ),
        LlmSection::table(
            "Key Commands",
            "| cmd | desc |\n|-----|------|\n| foo | bar |",
        ),
        LlmSection::raw("Raw appendix content that is optional.\n"),
    ]
}

// ── Token cost (task 2.1) ─────────────────────────────────────────────

#[test]
fn token_cost_is_labeled_not_mistaken_for_a_count() {
    let cost = estimate_token_cost("abcdefgh"); // 8 chars → chars/4 = 2
    assert_eq!(cost.estimate, 2);
    assert_eq!(cost.heuristic, "chars/4");
}

#[test]
fn estimate_is_ceil_of_chars_over_four() {
    assert_eq!(estimate_token_cost("").estimate, 0);
    assert_eq!(estimate_token_cost("a").estimate, 1); // ceil(1/4)
    assert_eq!(estimate_token_cost("abcde").estimate, 2); // ceil(5/4)
}

#[test]
fn adding_modules_increases_the_estimate() {
    let meta = fixture_meta();
    let before = est(&generate_llms_txt(&meta, &fixture_modules()));
    let mut more = fixture_modules();
    more.push(ModuleEntry::new("module_c", "Another module description."));
    let after = est(&generate_llms_txt(&meta, &more));
    assert!(
        after > before,
        "adding a module must strictly increase the estimate"
    );
}

// ── Existing signatures pinned (task 2.2) ─────────────────────────────

#[test]
fn existing_llms_txt_output_byte_identical_to_golden() {
    let golden = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/aix_llms_txt.txt"),
    )
    .unwrap();
    assert_eq!(
        generate_llms_txt(&fixture_meta(), &fixture_modules()),
        golden
    );
}

#[test]
fn existing_llm_txt_output_byte_identical_to_golden() {
    let golden = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/aix_llm_txt.txt"),
    )
    .unwrap();
    assert_eq!(
        generate_llm_txt("my-tool", "Top-level description.", &fixture_sections()),
        golden
    );
}

// ── Bounded generation (tasks 2.3–2.5) ────────────────────────────────

#[test]
fn bounded_llms_txt_under_budget_is_verbatim() {
    let full = generate_llms_txt(&fixture_meta(), &fixture_modules());
    assert_eq!(
        generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), est(&full)),
        full
    );
}

#[test]
fn bounded_llm_txt_under_budget_is_verbatim() {
    let full = generate_llm_txt("my-tool", "Top-level description.", &fixture_sections());
    assert_eq!(
        generate_llm_txt_bounded(
            "my-tool",
            "Top-level description.",
            &fixture_sections(),
            est(&full)
        ),
        full
    );
}

#[test]
fn over_budget_llms_txt_truncates_descriptions_first() {
    let full = generate_llms_txt(&fixture_meta(), &fixture_modules());
    let bounded = generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), est(&full) - 30);

    assert!(
        bounded.len() < full.len(),
        "degradation must shrink the artifact"
    );
    // Stage 1: descriptions truncated to first sentence…
    assert!(
        bounded.contains("- `module_a` — Handles the primary workload with care."),
        "descriptions truncated to first sentence; got:\n{bounded}"
    );
    // …but nothing structural dropped at this stage.
    assert!(bounded.contains("## Quick start"));
    assert!(bounded.contains("## Modules"));
}

#[test]
fn over_budget_llm_txt_truncates_description_before_dropping_sections() {
    // −10 lands inside stage 1 (heading-body truncation saves ~14 tokens).
    let full = generate_llm_txt("my-tool", "Top-level description.", &fixture_sections());
    let bounded = generate_llm_txt_bounded(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
        est(&full) - 10,
    );

    assert!(bounded.len() < full.len());
    // Stage 1 fires: descriptions truncated…
    assert!(
        bounded.contains("Top-level description."),
        "first sentence survives"
    );
    assert!(
        !bounded.contains("second sentence is detail"),
        "heading-body detail dropped"
    );
    // …but every section still present (nothing dropped at stage 1).
    assert!(bounded.contains("## Key Commands"));
    assert!(bounded.contains("| foo | bar |"));
    assert!(bounded.contains("Raw appendix content"));
}

#[test]
fn degradation_drops_raw_sections_before_tables() {
    // Budget lands between stage 1 and stage 2: raw gone, table intact.
    let full_est = est(&generate_llm_txt(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
    ));
    let bounded = generate_llm_txt_bounded(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
        full_est - 20,
    );
    assert!(
        !bounded.contains("Raw appendix content"),
        "raw dropped; got:\n{bounded}"
    );
    assert!(bounded.contains("| foo | bar |"), "table survives stage 2");
}

#[test]
fn headings_survive_all_degradation_levels() {
    // Budget lands between stage 2 and stage 3: tables dropped, headings kept.
    let full_est = est(&generate_llm_txt(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
    ));
    let bounded = generate_llm_txt_bounded(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
        full_est - 30,
    );
    assert!(bounded.contains("# my-tool"), "title survives");
    assert!(bounded.contains("## Overview"), "heading survives");
    assert!(
        bounded.contains("## Key Commands"),
        "heading survives even when table dropped"
    );
    assert!(!bounded.contains("| foo | bar |"), "table content dropped");
    assert!(!bounded.contains("Raw appendix"), "raw dropped");
}

#[test]
fn module_headings_survive_all_degradation_levels_llms() {
    let full_est = est(&generate_llms_txt(&fixture_meta(), &fixture_modules()));
    let bounded = generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), full_est - 50);
    assert!(bounded.contains("# my-tool"));
    assert!(bounded.contains("- `module_a`"), "module headings survive");
    assert!(bounded.contains("- `module_b`"));
}

#[test]
fn degradation_is_deterministic() {
    let full_llms = est(&generate_llms_txt(&fixture_meta(), &fixture_modules()));
    let full_llm = est(&generate_llm_txt(
        "my-tool",
        "Top-level description.",
        &fixture_sections(),
    ));
    for _ in 0..10 {
        let a = generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), full_llms - 30);
        let b = generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), full_llms - 30);
        assert_eq!(a, b);
        let c = generate_llm_txt_bounded(
            "my-tool",
            "Top-level description.",
            &fixture_sections(),
            full_llm - 20,
        );
        let d = generate_llm_txt_bounded(
            "my-tool",
            "Top-level description.",
            &fixture_sections(),
            full_llm - 20,
        );
        assert_eq!(c, d);
    }
}

#[test]
fn bounded_output_respects_budget_when_possible() {
    // The ladder must actually reach under budget — the minimal floor output
    // (title, tagline, links, bare module names) must fit a modest budget.
    let bounded = generate_llms_txt_bounded(&fixture_meta(), &fixture_modules(), 50);
    assert!(
        est(&bounded) <= 50,
        "bounded artifact must fit the declared budget; got {} tokens",
        est(&bounded)
    );
}
