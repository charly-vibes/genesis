//! Generation of `llms.txt` / `llm.txt` artifacts, with provenance footers
//! and token-budget degradation.

use std::io;
use std::path::Path;

use super::meta::{ModuleEntry, ProjectMeta, estimate_token_cost};
use super::sections::LlmSection;
use crate::managed_block::content_sha8;

pub fn generate_llms_txt(meta: &ProjectMeta, modules: &[ModuleEntry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n", meta.name));
    out.push('\n');

    // Tagline (skip if empty)
    if !meta.tagline.is_empty() {
        out.push_str(&format!("> {}\n", meta.tagline));
        out.push('\n');
    }

    let links = quick_start_links(meta);
    if !links.is_empty() {
        out.push_str("## Quick start\n\n");
        for (label, url) in &links {
            out.push_str(&format!("- [{}]({})\n", label, url));
        }
        out.push('\n');
    }

    let modules_list = render_module_list(modules);
    if !modules_list.is_empty() {
        out.push_str("## Modules\n\n");
        out.push_str(&modules_list);
    }

    out
}

/// Non-empty quick-start links of `meta`, in display order.
fn quick_start_links(meta: &ProjectMeta) -> Vec<(&'static str, &str)> {
    let mut links: Vec<(&'static str, &str)> = Vec::new();
    if let Some(repo) = meta.repository.as_deref().filter(|r| !r.is_empty()) {
        links.push(("Repository", repo));
    }
    if let Some(docs) = meta.documentation.as_deref().filter(|d| !d.is_empty()) {
        links.push(("Documentation", docs));
    }
    if let Some(crates) = meta.crates_io.as_deref().filter(|c| !c.is_empty()) {
        links.push(("crates.io", crates));
    }
    links
}

/// Markdown bullet list of `modules`; empty when there are none.
fn render_module_list(modules: &[ModuleEntry]) -> String {
    if modules.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for m in modules {
        if m.description.is_empty() {
            out.push_str(&format!("- `{}`\n", m.name));
        } else {
            out.push_str(&format!("- `{}` — {}\n", m.name, m.description));
        }
    }
    out.push('\n');
    out
}

/// Build the `# provenance:` footer line for a generated artifact.
///
/// `content` is the footer-free artifact text — the hash never covers the
/// footer itself, so the version bump / timestamp cannot shift it. Timestamp
/// goes last so the line stays grep-able in both variants.
fn aix_provenance_footer(content: &str, generated_at: Option<&str>) -> String {
    let mut line = format!(
        "# provenance: generator=genesis version={} sha={}",
        env!("CARGO_PKG_VERSION"),
        content_sha8(content)
    );
    if let Some(ts) = generated_at {
        line.push_str(&format!(" generated:{}", ts));
    }
    line
}

/// Append a `footer` line to `content` per the txt-footer convention:
/// blank-line separated, trailing newline guaranteed.
fn with_footer_line(content: &str, footer: String) -> String {
    let mut out = content.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&footer);
    out.push('\n');
    out
}

/// Generate `llms.txt` with a provenance footer (generator version + content
/// hash, no timestamp). Footer is appended after all content; the hash covers
/// only the footer-free content. Deterministic.
pub fn generate_llms_txt_with_provenance(meta: &ProjectMeta, modules: &[ModuleEntry]) -> String {
    let content = generate_llms_txt(meta, modules);
    with_footer_line(&content, aix_provenance_footer(&content, None))
}

/// Generate `llms.txt` with a provenance footer carrying a caller-supplied
/// timestamp (RFC 3339, e.g. audit trails). Deterministic for identical
/// inputs including the timestamp.
pub fn generate_llms_txt_timestamped(
    meta: &ProjectMeta,
    modules: &[ModuleEntry],
    generated_at: &str,
) -> String {
    let content = generate_llms_txt(meta, modules);
    with_footer_line(
        &content,
        aix_provenance_footer(&content, Some(generated_at)),
    )
}

/// Generate `llm.txt` with a provenance footer (no timestamp). Deterministic.
pub fn generate_llm_txt_with_provenance(
    title: &str,
    description: &str,
    sections: &[LlmSection],
) -> String {
    let content = generate_llm_txt(title, description, sections);
    with_footer_line(&content, aix_provenance_footer(&content, None))
}

/// Generate `llm.txt` with a provenance footer carrying a caller-supplied
/// timestamp (RFC 3339). Deterministic for identical inputs.
pub fn generate_llm_txt_timestamped(
    title: &str,
    description: &str,
    sections: &[LlmSection],
    generated_at: &str,
) -> String {
    let content = generate_llm_txt(title, description, sections);
    with_footer_line(
        &content,
        aix_provenance_footer(&content, Some(generated_at)),
    )
}

/// Write a complete `llms.txt` file at the given path.
///
/// Creates or overwrites the file. Returns the number of bytes written.
pub fn write_llms_txt(
    path: &Path,
    meta: &ProjectMeta,
    modules: &[ModuleEntry],
) -> io::Result<usize> {
    let content = generate_llms_txt(meta, modules);
    std::fs::write(path, &content)?;
    Ok(content.len())
}

// ── llm.txt generation ────────────────────────────────────────────────────────

/// Generate a complete `llm.txt` from a title, description paragraph,
/// and a list of sections.
///
/// The output starts with a `# Title` and description, then each section
/// is rendered in order.
///
/// # Example
///
/// ```
/// # use genesis::aix::{generate_llm_txt, LlmSection};
/// let doc = generate_llm_txt(
///     "my-tool",
///     "A brief description of the tool.",
///     &[
///         LlmSection::table("Key Commands", "| Command | Purpose |\n|---------|---------|\n| `check` | Run checks |"),
///         LlmSection::raw("## Links\n\n- [Repo](https://example.com)\n"),
///     ],
/// );
/// assert!(doc.starts_with("# my-tool"));
/// assert!(doc.contains("## Key Commands"));
/// ```
pub fn generate_llm_txt(title: &str, description: &str, sections: &[LlmSection]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", title));

    out.push_str(description);
    out.push('\n');
    out.push('\n');

    for section in sections {
        render_section(&mut out, section);
    }

    out
}

/// Render one `LlmSection` onto `out`, normalizing trailing newlines.
fn render_section(out: &mut String, section: &LlmSection) {
    match section {
        LlmSection::Heading { heading, body }
        | LlmSection::Table {
            heading,
            table: body,
        } => {
            out.push_str(&format!("## {heading}\n\n"));
            push_text(out, body);
            out.push('\n');
        }
        LlmSection::Raw(content) => push_text(out, content),
    }
}

/// Append `text`, guaranteeing a trailing newline.
fn push_text(out: &mut String, text: &str) {
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
}

// ── Token cost + bounded generation ─────────────────────────────────────

/// A heuristic token-cost estimate for a generated artifact.
///
/// `heuristic` names the approximation (`chars/4`) so callers can never
/// mistake the estimate for a tokenizer count. The chars/4 heuristic carries
/// a documented **±25% error band** against typical English prose in

fn first_sentence(s: &str) -> &str {
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
pub fn generate_llms_txt_bounded(
    meta: &ProjectMeta,
    modules: &[ModuleEntry],
    budget: usize,
) -> String {
    let full = generate_llms_txt(meta, modules);
    if estimate_token_cost(&full).estimate <= budget {
        return full;
    }

    let stage1: Vec<ModuleEntry> = modules
        .iter()
        .map(|m| ModuleEntry::new(&m.name, first_sentence(&m.description)))
        .collect();
    let truncated = generate_llms_txt(meta, &stage1);
    if estimate_token_cost(&truncated).estimate <= budget {
        return truncated;
    }

    let stage2: Vec<ModuleEntry> = modules
        .iter()
        .map(|m| ModuleEntry::new(&m.name, ""))
        .collect();
    generate_llms_txt(meta, &stage2)
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
pub fn generate_llm_txt_bounded(
    title: &str,
    description: &str,
    sections: &[LlmSection],
    budget: usize,
) -> String {
    let full = generate_llm_txt(title, description, sections);
    if estimate_token_cost(&full).estimate <= budget {
        return full;
    }

    let short_description = first_sentence(description);

    let stage1 = truncate_section_bodies(sections);
    let out1 = generate_llm_txt(title, short_description, &stage1);
    if estimate_token_cost(&out1).estimate <= budget {
        return out1;
    }

    let stage2 = drop_raw_sections(&stage1);
    let out2 = generate_llm_txt(title, short_description, &stage2);
    if estimate_token_cost(&out2).estimate <= budget {
        return out2;
    }

    let stage3 = tables_to_headings(&stage2);
    generate_llm_txt(title, short_description, &stage3)
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

/// Write a complete `llm.txt` file at the given path.
///
/// Creates or overwrites the file. Returns the number of bytes written.
pub fn write_llm_txt(
    path: &Path,
    title: &str,
    description: &str,
    sections: &[LlmSection],
) -> io::Result<usize> {
    let content = generate_llm_txt(title, description, sections);
    std::fs::write(path, &content)?;
    Ok(content.len())
}
