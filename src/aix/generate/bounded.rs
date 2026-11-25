//! Token-budget bounded generation with a deterministic degradation ladder
//! (design D2): the artifact shrinks in fixed granularity steps instead of
//! overflowing or being refused.

use super::footer::{aix_provenance_footer, with_footer_line};
use super::{generate_llm_txt, generate_llms_txt};
use crate::aix::meta::{ModuleEntry, ProjectMeta, estimate_token_cost};
use crate::aix::sections::LlmSection;

/// `true` when `artifact` fits within the `budget` (chars/4 token estimate).
fn fits(artifact: &str, budget: usize) -> bool {
    estimate_token_cost(artifact).estimate <= budget
}

/// Return the first sentence of `s`: everything up to and including the
/// first `". "` delimiter, or the whole trimmed string when there is none.
pub fn first_sentence(s: &str) -> &str {
    let trimmed = s.trim();
    match trimmed.find(". ") {
        Some(i) => trimmed[..i + 1].trim_end(),
        None => trimmed,
    }
}

/// Generate `llms.txt` under a declared token budget (chars/4 estimate).
///
/// Deterministic degradation ladder (design D2) — the artifact shrinks in
/// fixed granularity steps instead of overflowing or being refused:
///
/// 1. under budget → byte-identical to [`generate_llms_txt`]
/// 2. truncate module descriptions to their first sentence
/// 3. drop module descriptions entirely — module names (headings) survive
///
/// Stage 3 is the floor: the minimal artifact is returned even if it still
/// exceeds the budget (a verbose artifact beats a missing one).
///
/// Sentence truncation uses the plain `". "` delimiter — abbreviations like
/// `"e.g."` may end a "sentence" early, acceptable for a heuristic
/// (deterministic by construction).
///
/// Copy `modules`, deriving each description via `desc`.
fn with_descriptions(modules: &[ModuleEntry], desc: impl Fn(&str) -> String) -> Vec<ModuleEntry> {
    modules
        .iter()
        .map(|m| ModuleEntry::new(&m.name, &desc(&m.description)))
        .collect()
}

pub fn generate_llms_txt_bounded(
    meta: &ProjectMeta,
    modules: &[ModuleEntry],
    budget: usize,
) -> String {
    let full = generate_llms_txt(meta, modules);
    if fits(&full, budget) {
        return full;
    }
    let truncated = with_descriptions(modules, |d| first_sentence(d).to_string());
    let stage1 = generate_llms_txt(meta, &truncated);
    if fits(&stage1, budget) {
        return stage1;
    }
    let bare = with_descriptions(modules, |_| String::new());
    generate_llms_txt(meta, &bare)
}

/// [`generate_llms_txt_bounded`] with a provenance footer.
///
/// Degradation runs first (unchanged ladder); the footer is appended last and
/// its hash reflects the *degraded* content, so drift checks compare the
/// artifact as shipped. Deterministic.
pub fn generate_llms_txt_bounded_with_provenance(
    meta: &ProjectMeta,
    modules: &[ModuleEntry],
    budget: usize,
) -> String {
    let content = generate_llms_txt_bounded(meta, modules, budget);
    with_footer_line(&content, aix_provenance_footer(&content, None))
}

/// Generate `llm.txt` under a declared token budget (chars/4 estimate).
///
/// Deterministic degradation ladder (design D2):
///
/// 1. under budget → byte-identical to [`generate_llm_txt`]
/// 2. truncate the description and heading bodies to their first sentences
/// 3. drop raw (optional) sections — before any table is touched
/// 4. drop table content, keeping table headings — headings survive all
///    degradation levels
///
/// Stage 4 is the floor: returned even if still over budget.
///
/// Sentence truncation uses the plain `". "` delimiter — abbreviations like
/// `"e.g."` may end a "sentence" early, acceptable for a heuristic
/// (deterministic by construction).
///
/// Render one degradation stage; `Some` when it fits the `budget`.
fn stage_fits(
    title: &str,
    short_description: &str,
    sections: &[LlmSection],
    budget: usize,
) -> Option<String> {
    let out = generate_llm_txt(title, short_description, sections);
    fits(&out, budget).then_some(out)
}

pub fn generate_llm_txt_bounded(
    title: &str,
    description: &str,
    sections: &[LlmSection],
    budget: usize,
) -> String {
    let full = generate_llm_txt(title, description, sections);
    if fits(&full, budget) {
        return full;
    }
    let short_description = first_sentence(description);
    let stage1 = truncate_section_bodies(sections);
    if let Some(out) = stage_fits(title, short_description, &stage1, budget) {
        return out;
    }
    llm_later_stages(title, short_description, &stage1, budget)
}

/// Stages 3–4 of the `llm.txt` ladder: drop raw sections, then collapse
/// tables to headings (the floor).
fn llm_later_stages(
    title: &str,
    short_description: &str,
    stage1: &[LlmSection],
    budget: usize,
) -> String {
    let stage2 = drop_raw_sections(stage1);
    if let Some(out) = stage_fits(title, short_description, &stage2, budget) {
        return out;
    }
    generate_llm_txt(title, short_description, &tables_to_headings(&stage2))
}

/// Copy `sections`, truncating heading/table bodies to their first sentence.
fn truncate_section_bodies(sections: &[LlmSection]) -> Vec<LlmSection> {
    sections
        .iter()
        .map(|s| match s {
            LlmSection::Heading { heading, body } => LlmSection::Heading {
                heading: heading.clone(),
                body: first_sentence(body).to_string(),
            },
            other => other.clone(),
        })
        .collect()
}

/// Drop `Raw` sections (optional content goes first in the ladder).
fn drop_raw_sections(sections: &[LlmSection]) -> Vec<LlmSection> {
    sections
        .iter()
        .filter(|s| !matches!(s, LlmSection::Raw(_)))
        .cloned()
        .collect()
}

/// Collapse tables to bare headings with empty bodies (degradation floor).
fn tables_to_headings(sections: &[LlmSection]) -> Vec<LlmSection> {
    sections
        .iter()
        .map(|s| match s {
            LlmSection::Table { heading, .. } => LlmSection::Heading {
                heading: heading.clone(),
                body: String::new(),
            },
            other => other.clone(),
        })
        .collect()
}
