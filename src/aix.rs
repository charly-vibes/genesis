//! AIX artifact generation (`llms.txt`/`llm.txt`/agent blocks).
//!
//! Provides structured data types and generation functions for the
//! standardized `llms.txt` (concise project summary) and `llm.txt`
//! (detailed project context) files used by the charly-vibes suite.
//!
//! Tools compose their `llm.txt` from [`LlmSection`] values, using the
//! helper functions for common section types (modules, commands, genesis
//! adoption, authorship, links), then fill in tool-specific content.
//!
//! ## Self-hosting
//!
//! Genesis generates its own `llms.txt` and `llm.txt` via this module.
//! Run `just aix-gen` to regenerate them from the current module metadata.
//! See `justfile` in the repo root for the regeneration command.

use std::io;
use std::path::Path;

// ── Data types ─────────────────────────────────────────────────────────────────

/// Metadata describing a project for AIX file generation.
///
/// Used by [`generate_llms_txt()`] and helpers for `llm.txt` sections.
#[derive(Debug, Clone)]
pub struct ProjectMeta {
    /// Project name (e.g., `"genesis-vibes"`).
    pub name: String,
    /// Short tagline/one-liner.
    pub tagline: String,
    /// Optional repository URL.
    pub repository: Option<String>,
    /// Optional documentation URL.
    pub documentation: Option<String>,
    /// Optional crates.io URL.
    pub crates_io: Option<String>,
}

impl ProjectMeta {
    /// Create a new project metadata with the minimal fields.
    pub fn new(name: &str, tagline: &str) -> Self {
        Self {
            name: name.to_string(),
            tagline: tagline.to_string(),
            repository: None,
            documentation: None,
            crates_io: None,
        }
    }

    /// Builder-style: set the repository URL.
    pub fn with_repository(mut self, url: &str) -> Self {
        self.repository = Some(url.to_string());
        self
    }

    /// Builder-style: set the documentation URL.
    pub fn with_documentation(mut self, url: &str) -> Self {
        self.documentation = Some(url.to_string());
        self
    }

    /// Builder-style: set the crates.io URL.
    pub fn with_crates_io(mut self, url: &str) -> Self {
        self.crates_io = Some(url.to_string());
        self
    }
}

/// A module entry for listing in AIX artifacts.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// Module name (e.g., `"envelope"`).
    pub name: String,
    /// One-line description of the module.
    pub description: String,
}

impl ModuleEntry {
    /// Create a new module entry.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// A named section in an `llm.txt` document.
///
/// Variants cover the common patterns seen across the charly-vibes suite.
/// The [`generate_llm_txt()`] function renders them in order.
///
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum LlmSection {
    /// `## Heading` followed by body text (one or more paragraphs).
    Heading { heading: String, body: String },
    /// A markdown table preceded by a heading.
    Table { heading: String, table: String },
    /// Raw markdown content, inserted verbatim.
    Raw(String),
}

impl LlmSection {
    /// Create a heading section.
    pub fn heading(heading: &str, body: &str) -> Self {
        LlmSection::Heading {
            heading: heading.to_string(),
            body: body.to_string(),
        }
    }

    /// Create a table section.
    pub fn table(heading: &str, table: &str) -> Self {
        LlmSection::Table {
            heading: heading.to_string(),
            table: table.to_string(),
        }
    }

    /// Create a raw section (content inserted verbatim).
    pub fn raw(content: &str) -> Self {
        LlmSection::Raw(content.to_string())
    }
}

// ── llms.txt generation ───────────────────────────────────────────────────────

/// Generate a complete `llms.txt` from project metadata and module entries.
///
/// The output follows the standardized concise format used across the
/// charly-vibes suite: a `# Title` with tagline, quick-start links,
/// and a module listing.
///
/// # Example output structure
///
/// ```text
/// # my-tool
///
/// > One-line tagline.
///
/// ## Quick start
///
/// - [Repository](...)
/// - [Documentation](...)
/// - [crates.io](...)
///
/// ## Modules
///
/// - `module_a` — description a
/// - `module_b` — description b
/// ```
pub fn generate_llms_txt(meta: &ProjectMeta, modules: &[ModuleEntry]) -> String {
    let mut out = String::new();

    // Title
    out.push_str(&format!("# {}\n", meta.name));
    out.push('\n');

    // Tagline (skip if empty)
    if !meta.tagline.is_empty() {
        out.push_str(&format!("> {}\n", meta.tagline));
        out.push('\n');
    }

    // Quick start links (skip empty URLs)
    let mut links: Vec<(&str, &str)> = Vec::new();
    if let Some(repo) = meta.repository.as_deref().filter(|r| !r.is_empty()) {
        links.push(("Repository", repo));
    }
    if let Some(docs) = meta.documentation.as_deref().filter(|d| !d.is_empty()) {
        links.push(("Documentation", docs));
    }
    if let Some(crates) = meta.crates_io.as_deref().filter(|c| !c.is_empty()) {
        links.push(("crates.io", crates));
    }

    if !links.is_empty() {
        out.push_str("## Quick start\n");
        out.push('\n');
        for (label, url) in &links {
            out.push_str(&format!("- [{}]({})\n", label, url));
        }
        out.push('\n');
    }

    // Module listing
    if !modules.is_empty() {
        out.push_str("## Modules\n");
        out.push('\n');
        for m in modules {
            if m.description.is_empty() {
                out.push_str(&format!("- `{}`\n", m.name));
            } else {
                out.push_str(&format!("- `{}` — {}\n", m.name, m.description));
            }
        }
        out.push('\n');
    }

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

    // Title
    out.push_str(&format!("# {}\n", title));
    out.push('\n');

    // Description
    out.push_str(description);
    if !description.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    // Sections
    for section in sections {
        match section {
            LlmSection::Heading { heading, body } => {
                out.push_str(&format!("## {}\n", heading));
                out.push('\n');
                out.push_str(body);
                if !body.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            LlmSection::Table { heading, table } => {
                out.push_str(&format!("## {}\n", heading));
                out.push('\n');
                out.push_str(table);
                if !table.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            LlmSection::Raw(content) => {
                out.push_str(content);
                if !content.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }

    out
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

// ── Section helpers ───────────────────────────────────────────────────────────

/// Generate a "## Modules" section with a markdown table.
///
/// Produces a two-column table: module name and description.
///
/// # Example
///
/// ```text
/// ## Modules
///
/// | Module | Description |
/// |--------|-------------|
/// | `foo` | The foo module |
/// | `bar` | The bar module |
/// ```
pub fn modules_section(modules: &[ModuleEntry]) -> LlmSection {
    let mut table = String::from("| Module | Description |\n|--------|-------------|\n");
    for m in modules {
        table.push_str(&format!("| `{}` | {} |\n", m.name, m.description));
    }
    LlmSection::table("Modules", &table)
}

/// Generate a "## Key Commands" section with a markdown table.
///
/// Each command is a `(command_string, purpose_string)` pair.
///
/// # Example
///
/// ```text
/// ## Key Commands
///
/// | Command | Purpose |
/// |---------|---------|
/// | `init` | Initialize project |
/// | `check` | Run checks |
/// ```
pub fn commands_section(commands: &[(&str, &str)]) -> LlmSection {
    let mut table = String::from("| Command | Purpose |\n|---------|---------|\n");
    for (cmd, purpose) in commands {
        table.push_str(&format!("| `{}` | {} |\n", cmd, purpose));
    }
    LlmSection::table("Key Commands", &table)
}

/// Generate a genesis-adoption table for `llm.txt`.
///
/// Each entry is a `(module_name, usage_summary)` pair.
/// Commonly used in llm.txt to document which genesis modules a tool adopts.
///
/// # Example
///
/// ```text
/// ## Genesis adoption
///
/// | Module | Usage |
/// |--------|-------|
/// | `envelope` | Structured CLI output |
/// | `config` | Config file management |
/// ```
pub fn genesis_adoption_section(entries: &[(&str, &str)]) -> LlmSection {
    let mut table = String::from("| Module | Usage |\n|--------|-------|\n");
    for (name, usage) in entries {
        table.push_str(&format!("| `{}` | {} |\n", name, usage));
    }
    LlmSection::table("Genesis adoption", &table)
}

/// Generate an authorship attribution section.
///
/// `spanish_link` is an optional URL to the Spanish-language project overview
/// (common convention in the charly-vibes suite).
pub fn authorship_section(spanish_link: Option<&str>) -> LlmSection {
    let mut body = String::from(
        "All code in this repository was generated by a large language model. \
         Human contribution: requirements, design decisions, planning, and review.",
    );
    if let Some(url) = spanish_link {
        body.push_str(&format!("\n\nLeer en español: {}", url));
    }
    LlmSection::heading("Authorship", &body)
}

/// Generate a links section with a custom heading.
///
/// Each link is a `(label, url)` pair rendered as a bullet list.
///
/// # Example
///
/// ```text
/// ## Quick start
///
/// - [Repository](https://example.com)
/// - [Docs](https://docs.example.com)
/// ```
pub fn links_section(heading: &str, links: &[(&str, &str)]) -> LlmSection {
    let mut body = String::new();
    for (label, url) in links {
        body.push_str(&format!("- [{}]({})\n", label, url));
    }
    LlmSection::heading(heading, &body)
}

// ── agents_block (preserved from original) ─────────────────────────────────────

/// Generate a managed block for an agent instructions file.
///
/// This is a convenience wrapper around `managed_block::BlockInjector`.
/// It creates a block with the standard `<!-- NAME:START -->`/`<!-- NAME:END -->`
/// markers.
pub fn agents_block(name: &str, body: &str) -> String {
    format!("<!-- {}:START -->\n{}\n<!-- {}:END -->", name, body, name)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn test_meta() -> ProjectMeta {
        ProjectMeta::new(
            "genesis-vibes",
            "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure.",
        )
        .with_repository("https://github.com/charly-vibes/genesis")
        .with_documentation("https://charly-vibes.github.io/genesis/")
        .with_crates_io("https://crates.io/crates/genesis-vibes")
    }

    fn test_modules() -> Vec<ModuleEntry> {
        vec![
            ModuleEntry::new("envelope", "structured CLI output envelope"),
            ModuleEntry::new("suggestions", "self-healing error suggestions"),
            ModuleEntry::new("aix", "AIX artifact generation"),
        ]
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
        assert_eq!(m.repository.unwrap(), "https://example.com");
        assert_eq!(m.documentation.unwrap(), "https://docs.example.com");
        assert_eq!(m.crates_io.unwrap(), "https://crates.io/crates/foo");
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
    }

    #[test]
    fn test_generate_llms_txt_includes_tagline() {
        let output = generate_llms_txt(&test_meta(), &[]);
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

    #[test]
    fn test_generate_llms_txt_no_links_when_none_set() {
        let meta = ProjectMeta::new("minimal", "Minimal project.");
        let output = generate_llms_txt(&meta, &[]);
        assert!(!output.contains("## Quick start"));
    }

    #[test]
    fn test_generate_llms_txt_no_modules_when_empty() {
        let meta = ProjectMeta::new("empty", "Empty project.");
        let output = generate_llms_txt(&meta, &[]);
        assert!(!output.contains("## Modules"));
    }

    #[test]
    fn test_generate_llms_txt_skips_empty_tagline() {
        let meta = ProjectMeta::new("no-tagline", "");
        let output = generate_llms_txt(&meta, &[]);
        assert!(!output.contains(">"));
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
        let bytes = write_llms_txt(&path, &test_meta(), &test_modules()).unwrap();
        assert!(bytes > 0);
        assert!(path.exists());
    }

    #[test]
    fn test_write_llms_txt_content_matches_generate() {
        let dir = tmp();
        let path = dir.path().join("llms.txt");
        write_llms_txt(&path, &test_meta(), &test_modules()).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        let expected = generate_llms_txt(&test_meta(), &test_modules());
        assert_eq!(written, expected);
    }

    #[test]
    fn test_write_llms_txt_overwrites_existing() {
        let dir = tmp();
        let path = dir.path().join("llms.txt");
        write_llms_txt(&path, &test_meta(), &test_modules()).unwrap();
        // Write again with different content
        let meta2 = ProjectMeta::new("v2", "Version 2.");
        write_llms_txt(&path, &meta2, &[]).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# v2"));
        assert!(!content.contains("# genesis-vibes"));
    }

    // ── generate_llm_txt ──────────────────────────────────────────────

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
        let first = doc.find("## First").unwrap();
        let second = doc.find("## Second").unwrap();
        assert!(first < second, "sections in wrong order");
    }

    // ── write_llm_txt ─────────────────────────────────────────────────

    #[test]
    fn test_write_llm_txt_creates_file() {
        let dir = tmp();
        let path = dir.path().join("llm.txt");
        let bytes = write_llm_txt(&path, "t", "desc", &[]).unwrap();
        assert!(bytes > 0);
        assert!(path.exists());
    }

    #[test]
    fn test_write_llm_txt_content_matches_generate() {
        let dir = tmp();
        let path = dir.path().join("llm.txt");
        let sections = &[LlmSection::heading("Foo", "Bar")];
        write_llm_txt(&path, "my-tool", "desc", sections).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
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
                assert!(
                    body.contains("Leer en español: https://charly-vibes.github.io/charly-vibes/")
                );
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
        let start = block.find("<!-- TEST:START -->").unwrap();
        let end = block.find("<!-- TEST:END -->").unwrap();
        assert!(start < end, "START marker must precede END marker");
    }

    #[test]
    fn test_agents_block_roundtrip() {
        let body = "## Tool\n\nRun `tool check`.\n";
        let block = agents_block("DONT", body);
        assert!(block.contains(body));
    }

    // ── Integration: round-trip to file then read back ────────────────

    #[test]
    fn test_full_llms_roundtrip() {
        let dir = tmp();
        let path = dir.path().join("llms.txt");
        write_llms_txt(&path, &test_meta(), &test_modules()).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

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
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

        assert!(content.starts_with("# genesis-vibes"));
        assert!(content.contains("## Modules"));
        assert!(content.contains("## Key Commands"));
        assert!(content.contains("## Authorship"));
        assert!(content.contains("| `envelope` | structured CLI output envelope |"));
        assert!(content.contains("| `check` | Run checks |"));
    }
}
