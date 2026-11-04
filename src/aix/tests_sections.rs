// pretender: role=test
//! Section-builder, `llm.txt`, and agents-block tests for the `aix` module
//! (split from `tests.rs` for file-size hygiene).

use super::generate::{generate_llm_txt, write_llm_txt};
use super::meta::ModuleEntry;
use super::sections::{
    LlmSection, agents_block, authorship_section, commands_section, genesis_adoption_section,
    links_section, modules_section,
};
use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

#[test]
fn test_generate_llm_txt_title_first_line() {
    let doc = generate_llm_txt("my-tool", "A tool.", &[]);
    assert!(doc.starts_with("# my-tool"));
}

#[test]
fn test_generate_llm_txt_includes_description() {
    let doc = generate_llm_txt("t", "This is a description.", &[]);
    assert!(doc.contains("This is a description."));
}

#[test]
fn test_generate_llm_txt_includes_heading_section() {
    let sections = &[LlmSection::heading("Usage", "Run `tool check`.")];
    let doc = generate_llm_txt("t", "desc", sections);
    assert!(doc.contains("## Usage"));
    assert!(doc.contains("Run `tool check`."));
}

#[test]
fn test_generate_llm_txt_includes_table_section() {
    let sections = &[LlmSection::table(
        "Commands",
        "| cmd | purpose |\n|-----|---------|\n| `x` | do x |",
    )];
    let doc = generate_llm_txt("t", "desc", sections);
    assert!(doc.contains("## Commands"));
    assert!(doc.contains("| `x` | do x |"));
}

#[test]
fn test_generate_llm_txt_includes_raw_section() {
    let sections = &[LlmSection::raw("## Custom\n\nRaw content.")];
    let doc = generate_llm_txt("t", "desc", sections);
    assert!(doc.contains("## Custom"));
    assert!(doc.contains("Raw content."));
}

#[test]
fn test_generate_llm_txt_multiple_sections_in_order() {
    let sections = &[
        LlmSection::heading("First", "First body"),
        LlmSection::heading("Second", "Second body"),
    ];
    let doc = generate_llm_txt("t", "desc", sections);
    let first = doc.find("## First").require("value");
    let second = doc.find("## Second").require("value");
    assert!(first < second, "sections in wrong order");
}

// ── write_llm_txt ─────────────────────────────────────────────────

#[test]
fn test_write_llm_txt_creates_file() {
    let dir = tmp();
    let path = dir.path().join("llm.txt");
    let bytes = write_llm_txt(&path, "t", "desc", &[]).require("value");
    assert!(bytes > 0);
    assert!(path.exists());
}

#[test]
fn test_write_llm_txt_content_matches_generate() {
    let dir = tmp();
    let path = dir.path().join("llm.txt");
    let sections = &[LlmSection::heading("Foo", "Bar")];
    write_llm_txt(&path, "my-tool", "desc", sections).require("value");
    let written = std::fs::read_to_string(&path).require("value");
    let expected = generate_llm_txt("my-tool", "desc", sections);
    assert_eq!(written, expected);
}

// ── Section helpers ───────────────────────────────────────────────

#[test]
fn test_modules_section_has_table_header() {
    let section = modules_section(&[]);
    match section {
        LlmSection::Table {
            ref heading,
            ref table,
        } => {
            assert_eq!(heading, "Modules");
            assert!(table.contains("| Module | Description |"));
            assert!(table.contains("|--------|-------------|"));
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_modules_section_includes_entries() {
    let modules = vec![
        ModuleEntry::new("foo", "Foo module"),
        ModuleEntry::new("bar", "Bar module"),
    ];
    let section = modules_section(&modules);
    match section {
        LlmSection::Table { ref table, .. } => {
            assert!(table.contains("| `foo` | Foo module |"));
            assert!(table.contains("| `bar` | Bar module |"));
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_commands_section_has_table_header() {
    let section = commands_section(&[]);
    match section {
        LlmSection::Table {
            ref heading,
            ref table,
        } => {
            assert_eq!(heading, "Key Commands");
            assert!(table.contains("| Command | Purpose |"));
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_commands_section_includes_entries() {
    let section = commands_section(&[("init", "Initialize"), ("check", "Run checks")]);
    match section {
        LlmSection::Table { ref table, .. } => {
            assert!(table.contains("| `init` | Initialize |"));
            assert!(table.contains("| `check` | Run checks |"));
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_genesis_adoption_section_has_table_header() {
    let section = genesis_adoption_section(&[]);
    match section {
        LlmSection::Table { ref heading, .. } => {
            assert_eq!(heading, "Genesis adoption");
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_genesis_adoption_section_includes_entries() {
    let section = genesis_adoption_section(&[
        ("envelope", "Structured output"),
        ("config", "Config loading"),
    ]);
    match section {
        LlmSection::Table { ref table, .. } => {
            assert!(table.contains("| `envelope` | Structured output |"));
            assert!(table.contains("| `config` | Config loading |"));
        }
        _ => panic!("expected Table variant"),
    }
}

#[test]
fn test_authorship_section_without_link() {
    let section = authorship_section(None);
    match section {
        LlmSection::Heading {
            ref heading,
            ref body,
        } => {
            assert_eq!(heading, "Authorship");
            assert!(body.contains("large language model"));
            assert!(!body.contains("Leer en español"));
        }
        _ => panic!("expected Heading variant"),
    }
}

#[test]
fn test_authorship_section_with_spanish_link() {
    let section = authorship_section(Some("https://charly-vibes.github.io/charly-vibes/"));
    match section {
        LlmSection::Heading { ref body, .. } => {
            assert!(body.contains("Leer en español: https://charly-vibes.github.io/charly-vibes/"));
        }
        _ => panic!("expected Heading variant"),
    }
}

#[test]
fn test_links_section_with_custom_heading() {
    let section = links_section(
        "Quick start",
        &[("Repo", "https://repo"), ("Docs", "https://docs")],
    );
    match section {
        LlmSection::Heading {
            ref heading,
            ref body,
        } => {
            assert_eq!(heading, "Quick start");
            assert!(body.contains("[Repo](https://repo)"));
            assert!(body.contains("[Docs](https://docs)"));
        }
        _ => panic!("expected Heading variant"),
    }
}

#[test]
fn test_links_section_different_heading() {
    let section = links_section("Links", &[("Home", "https://home")]);
    match section {
        LlmSection::Heading {
            ref heading,
            ref body,
        } => {
            assert_eq!(heading, "Links");
            assert!(body.contains("[Home](https://home)"));
        }
        _ => panic!("expected Heading variant"),
    }
}

// ── agents_block (preserved) ──────────────────────────────────────

#[test]
fn test_agents_block_generates_markers() {
    let block = agents_block("WAI", "# Content\n");
    assert!(block.contains("<!-- WAI:START -->"));
    assert!(block.contains("<!-- WAI:END -->"));
    assert!(block.contains("# Content"));
}

#[test]
fn test_agents_block_start_before_end() {
    let block = agents_block("TEST", "body");
    let start = block.find("<!-- TEST:START -->").require("value");
    let end = block.find("<!-- TEST:END -->").require("value");
    assert!(start < end, "START marker must precede END marker");
}

#[test]
fn test_agents_block_roundtrip() {
    let body = "## Tool\n\nRun `tool check`.\n";
    let block = agents_block("DONT", body);
    assert!(block.contains(body));
}

// ── Integration: round-trip to file then read back ────────────────
