//! Self-hosting AIX generation for genesis-vibes.
//!
//! Run via `just aix-gen` to regenerate `llms.txt` and `llm.txt`
//! from the current module metadata. This ensures the AIX artifacts
//! stay in sync with what the module actually provides.
//!
//! Usage:
//!   cargo run --example gen-aix

use genesis::aix::{
    ModuleEntry, ProjectMeta, authorship_section, commands_section, generate_llm_txt,
    generate_llms_txt, genesis_adoption_section, links_section, modules_section, write_llm_txt,
    write_llms_txt,
};

fn main() {
    let meta = ProjectMeta::new(
        "genesis-vibes",
        "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure.",
    )
    .with_repository("https://github.com/charly-vibes/genesis")
    .with_documentation("https://charly-vibes.github.io/genesis/")
    .with_crates_io("https://crates.io/crates/genesis-vibes");

    // Single source of truth for module metadata — used for both llms.txt and llm.txt
    let modules = vec![
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
    ];

    // Key types for adoption section — derived from same module data
    let key_types: Vec<(&str, &str)> = vec![
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
    ];

    // Generate llms.txt
    let llms_content = generate_llms_txt(&meta, &modules);
    let llms_path = std::path::Path::new("llms.txt");
    let stored_llms = std::fs::read_to_string(llms_path).unwrap_or_default();

    if llms_content != stored_llms {
        write_llms_txt(llms_path, &meta, &modules).expect("write llms.txt");
        println!("✓ llms.txt regenerated");
    } else {
        println!("✓ llms.txt unchanged");
    }

    // Generate llm.txt
    let sections = vec![
        modules_section(&modules),
        commands_section(&[(
            "(use via genesis)",
            "All modules are library APIs, not CLI subcommands",
        )]),
        genesis_adoption_section(&key_types),
        links_section(
            "Links",
            &[
                ("Repository", "https://github.com/charly-vibes/genesis"),
                ("Documentation", "https://charly-vibes.github.io/genesis/"),
                ("crates.io", "https://crates.io/crates/genesis-vibes"),
            ],
        ),
        authorship_section(Some("https://charly-vibes.github.io/charly-vibes/")),
    ];

    let llm_description = "Shared crate for cross-cutting CLI/AIX/self-healing infrastructure in the charly-vibes suite.";
    let llm_content = generate_llm_txt("genesis-vibes", llm_description, &sections);
    let llm_path = std::path::Path::new("llm.txt");
    let stored_llm = std::fs::read_to_string(llm_path).unwrap_or_default();

    if llm_content != stored_llm {
        write_llm_txt(llm_path, "genesis-vibes", llm_description, &sections)
            .expect("write llm.txt");
        println!("✓ llm.txt regenerated");
    } else {
        println!("✓ llm.txt unchanged");
    }
}
