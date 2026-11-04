//! Self-hosting AIX generation for genesis-vibes.
//!
//! Run via `just aix-gen` to regenerate `llms.txt` and `llm.txt`
//! from the current module metadata. This ensures the AIX artifacts
//! stay in sync with what the module actually provides.
//!
//! Usage:
//!   cargo run --example gen-aix

use genesis::aix::{
    LlmSection, ModuleEntry, ProjectMeta, authorship_section, commands_section, generate_llm_txt,
    generate_llms_txt, genesis_adoption_section, links_section, modules_section, write_llm_txt,
    write_llms_txt,
};

fn project_meta() -> ProjectMeta {
    ProjectMeta::new(
        "genesis-vibes",
        "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure.",
    )
    .with_repository("https://github.com/charly-vibes/genesis")
    .with_documentation("https://charly-vibes.github.io/genesis/")
    .with_crates_io("https://crates.io/crates/genesis-vibes")
}

/// Single source of truth for module metadata — used for both llms.txt and
/// llm.txt.
fn module_entries() -> Vec<ModuleEntry> {
    vec![
        ModuleEntry::new("envelope", "structured CLI output envelope"),
        ModuleEntry::new("suggestions", "self-healing error suggestions"),
        ModuleEntry::new("managed_block", "managed block injector"),
        ModuleEntry::new("aix", "AIX artifact generation"),
        ModuleEntry::new("config", "shared config management"),
        ModuleEntry::new(
            "guide",
            "CLI scaffold (Verbosity, CliVerbosity, OutputFormat, CliFormat, Output, ErrorSink, GuideBuilder, Guide). Format auto-detect: JSON for non-TTY stdout, Human for terminals.",
        ),
        ModuleEntry::new("fixture", "test scratch environments"),
        ModuleEntry::new("feedback", "agent issue reporting"),
        ModuleEntry::new("suite_linter", "suite-wide lint checks"),
        ModuleEntry::new("doctor", "diagnostics with auto-fix"),
        ModuleEntry::new("cli", "CLI helpers"),
        ModuleEntry::new("status", "cross-tool status dashboard"),
        ModuleEntry::new("scaffold", "init scaffolding"),
        ModuleEntry::new(
            "discovery",
            "tool discovery via .genesis/tools.toml manifest",
        ),
        ModuleEntry::new(
            "evals",
            "agent evals: ErrorTaxonomy, deterministic Scenario replay, eval report contract",
        ),
        ModuleEntry::new(
            "git",
            "shared read-only git plumbing: repo root, changed/uncommitted files, tracked/ignored, content hash",
        ),
        ModuleEntry::new(
            "git_hooks",
            "shared git hook primitives: install/uninstall with ownership markers, owner/framework detection, lefthook wiring",
        ),
    ]
}

/// Key types for the adoption section — derived from the same module data.
fn key_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("envelope", "Envelope, EnvelopeKind, ErrorResult"),
        ("suggestions", "Suggestion enum, SuggestionEngine"),
        ("managed_block", "<!-- …:START -->/<!-- …:END --> injector"),
        ("aix", "llms.txt/llm.txt generation, agent block injection"),
        ("config", "ConfigFile trait, ConfigRegistry, ConfigStore"),
        (
            "guide",
            "Verbosity, CliVerbosity, OutputFormat, CliFormat, Output, ErrorSink, GuideBuilder, Guide — CLI scaffold with progressive-disclosure verbosity, TTY-aware output format, and format-dispatching commands",
        ),
        ("fixture", "Fixture builder, assertions, Fixture::run"),
        ("feedback", "handle_feedback(), FeedbackArgs"),
        ("suite_linter", "LintCheck trait, LinterRegistry"),
        ("doctor", "DoctorCheck trait, DoctorRunner, DoctorReport"),
        ("cli", "generate_completions(), maybe_print_version_json()"),
        (
            "status",
            "StatusContributor trait, StatusBuilder, DoctorStatusBridge",
        ),
        ("scaffold", "Scaffold builder"),
        ("discovery", "scan(), register(), unregister(), Manifest"),
        (
            "evals",
            "ErrorTaxonomy, parse_envelope(), Scenario, ScenarioReport — deterministic in-process eval replay",
        ),
        (
            "git",
            "repo_root(), run(), changed_files(), uncommitted_files(), tracked(), is_ignored(), content_hash(), parse_porcelain()",
        ),
        (
            "git_hooks",
            "install(), uninstall(), owner(), framework(), resolve_hooks_dir(), lefthook::ensure_wired(), lefthook::is_wired()",
        ),
    ]
}

/// Regenerate `llms.txt` unless the stored artifact already matches.
fn regenerate_llms_txt(meta: &ProjectMeta, modules: &[ModuleEntry]) {
    let content = generate_llms_txt(meta, modules);
    let path = std::path::Path::new("llms.txt");
    let stored = std::fs::read_to_string(path).unwrap_or_default();

    if content != stored {
        write_llms_txt(path, meta, modules).expect("write llms.txt");
        println!("✓ llms.txt regenerated");
    } else {
        println!("✓ llms.txt unchanged");
    }
}

/// The llm.txt section list for the genesis suite.
fn llm_sections(modules: &[ModuleEntry]) -> Vec<LlmSection> {
    vec![
        modules_section(modules),
        commands_section(&[(
            "(use via genesis)",
            "All modules are library APIs, not CLI subcommands",
        )]),
        genesis_adoption_section(&key_types()),
        links_section(
            "Links",
            &[
                ("Repository", "https://github.com/charly-vibes/genesis"),
                ("Documentation", "https://charly-vibes.github.io/genesis/"),
                ("crates.io", "https://crates.io/crates/genesis-vibes"),
            ],
        ),
        authorship_section(Some("https://charly-vibes.github.io/charly-vibes/")),
    ]
}

/// Regenerate `llm.txt` unless the stored artifact already matches.
fn regenerate_llm_txt(modules: &[ModuleEntry]) {
    let description = "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure in the charly-vibes suite.";
    let sections = llm_sections(modules);
    let content = generate_llm_txt("genesis-vibes", description, &sections);
    let path = std::path::Path::new("llm.txt");
    let stored = std::fs::read_to_string(path).unwrap_or_default();

    if content != stored {
        write_llm_txt(path, "genesis-vibes", description, &sections).expect("write llm.txt");
        println!("✓ llm.txt regenerated");
    } else {
        println!("✓ llm.txt unchanged");
    }
}

fn main() {
    let meta = project_meta();
    let modules = module_entries();
    regenerate_llms_txt(&meta, &modules);
    regenerate_llm_txt(&modules);
}
