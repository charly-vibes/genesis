//! Generation of `llms.txt` / `llm.txt` artifacts, with provenance footers
//! and token-budget degradation.

use std::io;
use std::path::Path;

use super::meta::{ModuleEntry, ProjectMeta};
use super::sections::LlmSection;

mod bounded;
mod footer;

pub use bounded::{
    generate_llm_txt_bounded, generate_llms_txt_bounded, generate_llms_txt_bounded_with_provenance,
};
pub use footer::{
    generate_llm_txt_timestamped, generate_llm_txt_with_provenance, generate_llms_txt_timestamped,
    generate_llms_txt_with_provenance,
};

pub fn generate_llms_txt(meta: &ProjectMeta, modules: &[ModuleEntry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n", meta.name));
    out.push('\n');

    if !meta.tagline.is_empty() {
        out.push_str(&format!("> {}\n", meta.tagline));
        out.push('\n');
    }

    let links = quick_start_links(meta);
    push_quick_start(&mut out, &links);

    let modules_list = render_module_list(modules);
    push_modules(&mut out, &modules_list);

    out
}

/// Emit the `## Modules` header plus list when the list is non-empty.
fn push_modules(out: &mut String, modules_list: &str) {
    if modules_list.is_empty() {
        return;
    }
    out.push_str("## Modules\n\n");
    out.push_str(modules_list);
}

/// Render the `## Quick start` block for non-empty `links`.
fn push_quick_start(out: &mut String, links: &[(&'static str, &str)]) {
    if links.is_empty() {
        return;
    }
    out.push_str("## Quick start\n\n");
    for (label, url) in links {
        out.push_str(&format!("- [{}]({})\n", label, url));
    }
    out.push('\n');
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
