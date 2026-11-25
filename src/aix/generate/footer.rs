//! Provenance footers for `llms.txt` / `llm.txt` artifacts, plus the
//! `*_with_provenance` / `*_timestamped` generator variants.

use super::{generate_llm_txt, generate_llms_txt};
use crate::aix::meta::{ModuleEntry, ProjectMeta};
use crate::aix::sections::LlmSection;
use crate::managed_block::content_sha8;

/// Build the `# provenance:` footer line for a generated artifact.
///
/// `content` is the footer-free artifact text — the hash never covers the
/// footer itself, so the version bump / timestamp cannot shift it. Timestamp
/// goes last so the line stays grep-able in both variants.
pub(super) fn aix_provenance_footer(content: &str, generated_at: Option<&str>) -> String {
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
pub(super) fn with_footer_line(content: &str, footer: String) -> String {
    let mut out = content.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&footer);
    out.push('\n');
    out
}

/// Generate `llms.txt` with a provenance footer (generator version + content hash).
///
/// No timestamp. The footer is appended after all content; the hash covers only
/// the footer-free content. Deterministic.
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
