//! Tests for `aix` provenance footers and token-budget bounded
//! degradation (add-artifact-provenance, design D2).

// pretender: role=test
use super::generate::*;
use super::meta::{ModuleEntry, ProjectMeta, estimate_token_cost};
use super::sections::LlmSection;
use crate::managed_block::content_sha8;
use crate::testutil::TestUnwrap;

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

// ── Provenance footer (add-artifact-provenance §2) ─────────────────

/// RED 2.1: default generators stay deterministic and footer-free.

#[test]
fn test_default_generators_deterministic_and_footer_free() {
    let a = generate_llms_txt(&test_meta(), &test_modules());
    let b = generate_llms_txt(&test_meta(), &test_modules());
    assert_eq!(a, b, "llms.txt deterministic");
    assert!(!a.contains("provenance:"), "default llms.txt has no footer");
    assert!(
        !a.contains("generated:"),
        "default llms.txt has no timestamp"
    );

    let sections = vec![LlmSection::heading("Foo", "Bar body.")];
    let c = generate_llm_txt("t", "desc", &sections);
    let d = generate_llm_txt("t", "desc", &sections);
    assert_eq!(c, d, "llm.txt deterministic");
    assert!(!c.contains("provenance:"), "default llm.txt has no footer");
}

/// RED 2.2a: footer variant appends `# provenance:` as the final line;
/// hash covers content excluding the footer itself.

#[test]
fn test_llms_txt_footer_final_line_hash_excludes_footer() {
    let base = generate_llms_txt(&test_meta(), &test_modules());
    let with_footer = generate_llms_txt_with_provenance(&test_meta(), &test_modules());

    assert!(
        with_footer.starts_with(&base),
        "footer appends after content"
    );
    let last = with_footer.lines().last().require("value");
    assert!(last.starts_with("# provenance: "));
    assert!(last.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))));
    assert!(last.contains(&format!("sha={}", content_sha8(&base))));
    assert!(
        !last.contains("generated:"),
        "default variant is timestamp-free"
    );
}

/// RED 2.2b: llm.txt footer variant; hash over content excluding footer.

#[test]
fn test_llm_txt_footer_variant() {
    let sections = vec![LlmSection::heading("Foo", "Bar body.")];
    let base = generate_llm_txt("t", "desc", &sections);
    let with_footer = generate_llm_txt_with_provenance("t", "desc", &sections);

    assert!(with_footer.starts_with(&base));
    let last = with_footer.lines().last().require("value");
    assert!(last.starts_with("# provenance: "));
    assert!(last.contains(&format!("sha={}", content_sha8(&base))));
}

/// RED 2.2c: timestamped variant stamps a caller-supplied RFC 3339 time
/// (caller owns the clock — keeps generators hermetic).

#[test]
fn test_timestamped_variants_carry_provided_timestamp() {
    let ts = "2026-10-01T12:00:00Z";
    let llms = generate_llms_txt_timestamped(&test_meta(), &test_modules(), ts);
    let last_llms = llms.lines().last().require("value");
    assert!(last_llms.starts_with("# provenance: "));
    assert!(last_llms.contains(&format!("generated:{}", ts)));

    let sections = vec![LlmSection::heading("Foo", "Bar body.")];
    let llm = generate_llm_txt_timestamped("t", "desc", &sections, ts);
    let last_llm = llm.lines().last().require("value");
    assert!(last_llm.contains(&format!("generated:{}", ts)));
}

/// RED 2.3: bounded degradation stays deterministic and the footer hash
/// reflects the degraded content (not the full artifact's).

#[test]
fn test_bounded_degrades_deterministically_with_degraded_hash() {
    let long_desc = "A very long module description. With several sentences. Way over budget.";
    let modules = vec![ModuleEntry::new("m", long_desc)];
    let meta = ProjectMeta::new("tool", "tagline");
    let budget = estimate_token_cost("tiny").estimate; // 1 token — forces stage 3

    let a = generate_llms_txt_bounded_with_provenance(&meta, &modules, budget);
    let b = generate_llms_txt_bounded_with_provenance(&meta, &modules, budget);
    assert_eq!(a, b, "bounded+footer deterministic");
    assert!(a.contains("- `m`\n"), "module name survives degradation");
    assert!(
        !a.contains(long_desc),
        "description degraded away (stage 3)"
    );

    let last = a.lines().last().require("value");
    let footer_free = a
        .lines()
        .take(a.lines().count() - 1)
        .collect::<Vec<_>>()
        .join("\n");
    let expected_base = generate_llms_txt(&meta, &[ModuleEntry::new("m", "")]);
    assert_eq!(
        footer_free, expected_base,
        "degraded body matches ladder output"
    );
    assert!(last.contains(&format!("sha={}", content_sha8(&expected_base))));
}
