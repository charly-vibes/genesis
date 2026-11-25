//! Unit tests for the `aix` module (extracted from the inline `mod tests`
//! to keep app files under the pretender file-lines budget).

// pretender: role=test
use super::generate::*;
use super::meta::{ModuleEntry, ProjectMeta, estimate_token_cost};
use super::sections::{LlmSection, authorship_section, commands_section, modules_section};
use crate::managed_block::content_sha8;
use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

fn test_meta() -> ProjectMeta {
    let meta = ProjectMeta::new(
        "genesis-vibes",
        "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure.",
    )
    .with_repository("https://github.com/charly-vibes/genesis")
    .with_documentation("https://charly-vibes.github.io/genesis/")
    .with_crates_io("https://crates.io/crates/genesis-vibes");
    assert_eq!(
        meta.repository.as_deref(),
        Some("https://github.com/charly-vibes/genesis"),
        "shared fixture must carry the repository link"
    );
    meta
}

fn test_modules() -> Vec<ModuleEntry> {
    let modules = vec![
        ModuleEntry::new("envelope", "structured CLI output envelope"),
        ModuleEntry::new("suggestions", "self-healing error suggestions"),
        ModuleEntry::new("aix", "AIX artifact generation"),
    ];
    assert_eq!(
        modules.len(),
        3,
        "shared fixture must list exactly three modules"
    );
    modules
}

// ── ProjectMeta ───────────────────────────────────────────────────

#[test]
fn test_project_meta_new() {
    let m = ProjectMeta::new("foo", "A foo tool.");
    assert_eq!(m.name, "foo");
    assert_eq!(m.tagline, "A foo tool.");
    assert!(m.repository.is_none());
    assert!(m.documentation.is_none());
    assert!(m.crates_io.is_none());
}

#[test]
fn test_project_meta_builder() {
    let m = ProjectMeta::new("foo", "A tool.")
        .with_repository("https://example.com")
        .with_documentation("https://docs.example.com")
        .with_crates_io("https://crates.io/crates/foo");
    assert_eq!(m.repository.require("value"), "https://example.com");
    assert_eq!(m.documentation.require("value"), "https://docs.example.com");
    assert_eq!(m.crates_io.require("value"), "https://crates.io/crates/foo");
}

// ── ModuleEntry ───────────────────────────────────────────────────

#[test]
fn test_module_entry_new() {
    let m = ModuleEntry::new("foo", "The foo module");
    assert_eq!(m.name, "foo");
    assert_eq!(m.description, "The foo module");
}

// ── LlmSection ────────────────────────────────────────────────────

#[test]
fn test_llm_section_heading() {
    let s = LlmSection::heading("Foo", "Body text.");
    match s {
        LlmSection::Heading { heading, body } => {
            assert_eq!(heading, "Foo");
            assert_eq!(body, "Body text.");
        }
        _ => panic!("expected Heading variant"),
    }
}

#[test]
fn test_llm_section_table() {
    let s = LlmSection::table("Table", "| A | B |");
    match s {
        LlmSection::Table { heading, table } => {
            assert_eq!(heading, "Table");
            assert_eq!(table, "| A | B |");
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_llm_section_raw() {
    let s = LlmSection::raw("raw content");
    match s {
        LlmSection::Raw(c) => assert_eq!(c, "raw content"),
        _ => panic!("expected Raw variant"),
    }
}

// ── generate_llms_txt ─────────────────────────────────────────────

#[test]
fn test_generate_llms_txt_includes_title() {
    let output = generate_llms_txt(&test_meta(), &[]);
    assert!(output.contains("# genesis-vibes"));
    assert!(output.contains("> Shared crate for cross-cutting"));
}

#[test]
fn test_generate_llms_txt_includes_links() {
    let output = generate_llms_txt(&test_meta(), &[]);
    assert!(output.contains("## Quick start"));
    assert!(output.contains("[Repository](https://github.com/charly-vibes/genesis)"));
    assert!(output.contains("[Documentation](https://charly-vibes.github.io/genesis/)"));
    assert!(output.contains("[crates.io](https://crates.io/crates/genesis-vibes)"));
}

#[test]
fn test_generate_llms_txt_includes_modules() {
    let modules = test_modules();
    let output = generate_llms_txt(&test_meta(), &modules);
    assert!(output.contains("## Modules"));
    assert!(output.contains("`envelope` — structured CLI output envelope"));
    assert!(output.contains("`suggestions` — self-healing error suggestions"));
    assert!(output.contains("`aix` — AIX artifact generation"));
}

/// Omitted-content guards: absent links, absent modules, and an empty
/// tagline each leave their section/markup out of `llms.txt`.
#[test]
fn test_generate_llms_txt_omits_absent_content() {
    let empty_modules: &[ModuleEntry] = &[];
    let cases = [
        (
            "minimal",
            "Minimal project.",
            "## Quick start",
            empty_modules,
            "no links set",
        ),
        (
            "empty",
            "Empty project.",
            "## Modules",
            empty_modules,
            "no modules set",
        ),
        (
            "no-tagline",
            "",
            ">",
            empty_modules,
            "empty tagline skipped",
        ),
    ];
    for (name, tagline, absent, modules, why) in cases {
        let meta = ProjectMeta::new(name, tagline);
        let output = generate_llms_txt(&meta, modules);
        assert!(!output.contains(absent), "{why}");
    }
}

#[test]
fn test_generate_llms_txt_skips_empty_url() {
    let meta = ProjectMeta::new("tool", "A tool.").with_repository("");
    let output = generate_llms_txt(&meta, &[]);
    assert!(!output.contains("Repository"));
    assert!(!output.contains("[](#"));
}

#[test]
fn test_generate_llms_txt_handles_empty_description() {
    let modules = vec![
        ModuleEntry::new("foo", "Has description"),
        ModuleEntry::new("bar", ""),
    ];
    let meta = ProjectMeta::new("tool", "desc");
    let output = generate_llms_txt(&meta, &modules);
    assert!(output.contains("`foo` — Has description"));
    assert!(output.contains("- `bar`"));
    assert!(!output.contains("`bar` —"));
}

#[test]
fn test_generate_llms_txt_title_on_first_line() {
    let output = generate_llms_txt(&test_meta(), &[]);
    assert!(
        output.starts_with("# genesis-vibes"),
        "title should be first"
    );
}

// ── write_llms_txt ────────────────────────────────────────────────

#[test]
fn test_write_llms_txt_creates_file() {
    let dir = tmp();
    let path = dir.path().join("llms.txt");
    let bytes = write_llms_txt(&path, &test_meta(), &test_modules()).require("value");
    assert!(bytes > 0);
    assert!(path.exists());
}

#[test]
fn test_write_llms_txt_content_matches_generate() {
    let dir = tmp();
    let path = dir.path().join("llms.txt");
    write_llms_txt(&path, &test_meta(), &test_modules()).require("value");
    let written = std::fs::read_to_string(&path).require("value");
    let expected = generate_llms_txt(&test_meta(), &test_modules());
    assert_eq!(written, expected);
}

#[test]
fn test_write_llms_txt_overwrites_existing() {
    let dir = tmp();
    let path = dir.path().join("llms.txt");
    write_llms_txt(&path, &test_meta(), &test_modules()).require("value");
    // Write again with different content
    let meta2 = ProjectMeta::new("v2", "Version 2.");
    write_llms_txt(&path, &meta2, &[]).require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.contains("# v2"));
    assert!(!content.contains("# genesis-vibes"));
}

// ── generate_llm_txt ──────────────────────────────────────────────

#[test]
fn test_full_llms_roundtrip() {
    let dir = tmp();
    let path = dir.path().join("llms.txt");
    write_llms_txt(&path, &test_meta(), &test_modules()).require("value");
    let content = std::fs::read_to_string(&path).require("value");

    // Verify structure
    assert!(content.starts_with("# genesis-vibes"));
    assert!(content.contains("> Shared crate"));
    assert!(content.contains("## Quick start"));
    assert!(content.contains("## Modules"));
}

#[test]
fn test_full_llm_roundtrip() {
    let dir = tmp();
    let path = dir.path().join("llm.txt");
    let sections = &[
        modules_section(&test_modules()),
        commands_section(&[("check", "Run checks"), ("init", "Initialize")]),
        authorship_section(Some("https://charly-vibes.github.io/charly-vibes/")),
    ];
    write_llm_txt(
        &path,
        "genesis-vibes",
        "Shared crate description.",
        sections,
    )
    .require("value");
    let content = std::fs::read_to_string(&path).require("value");

    assert!(content.starts_with("# genesis-vibes"));
    assert!(content.contains("## Modules"));
    assert!(content.contains("## Key Commands"));
    assert!(content.contains("## Authorship"));
    assert!(content.contains("| `envelope` | structured CLI output envelope |"));
    assert!(content.contains("| `check` | Run checks |"));
}
