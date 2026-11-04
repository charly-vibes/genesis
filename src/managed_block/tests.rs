//! Unit tests for `managed_block` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget).

// pretender: role=test
#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

fn test_registry() -> BlockRegistry {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("DONT"));
    reg.register(BlockDef::with_markers(
        "ah:managed",
        "<!-- ah:managed:start -->",
        "<!-- ah:managed:end -->",
    ));
    assert_eq!(
        reg.names().len(),
        3,
        "shared fixture must register exactly three blocks"
    );
    reg
}

fn test_injector() -> BlockInjector {
    let injector = BlockInjector::new(test_registry());
    assert_eq!(
        injector.registry.names().len(),
        3,
        "shared injector fixture must carry the three shared blocks"
    );
    injector
}

// ── BlockDef ──────────────────────────────────────────────────────

#[test]
fn test_block_def_new_generates_markers() {
    let b = BlockDef::new("WAI");
    assert_eq!(b.name, "WAI");
    assert_eq!(b.start_marker, "<!-- WAI:START -->");
    assert_eq!(b.end_marker, "<!-- WAI:END -->");
}

#[test]
fn test_block_def_with_custom_markers() {
    let b = BlockDef::with_markers(
        "ah:managed",
        "<!-- ah:managed:start -->",
        "<!-- ah:managed:end -->",
    );
    assert_eq!(b.name, "ah:managed");
    assert_eq!(b.start_marker, "<!-- ah:managed:start -->");
    assert_eq!(b.end_marker, "<!-- ah:managed:end -->");
}

// ── BlockRegistry ─────────────────────────────────────────────────

#[test]
fn test_registry_empty_by_default() {
    let reg = BlockRegistry::new();
    assert!(reg.names().is_empty());
}

#[test]
fn test_registry_register_and_retrieve() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    assert!(reg.has("WAI"));
    assert_eq!(reg.get("WAI").require("value").name, "WAI");
}

#[test]
fn test_registry_replace_existing() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("WAI")); // replace
    assert_eq!(reg.names().len(), 1);
}

#[test]
fn test_registry_multiple_blocks() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("DONT"));
    reg.register(BlockDef::new("OPENSPEC"));
    assert_eq!(reg.names().len(), 3);
}

#[test]
fn test_registry_get_unknown() {
    let reg = BlockRegistry::new();
    assert!(reg.get("UNKNOWN").is_none());
}

// ── BlockInjector: create new file ─────────────────────────────────

#[test]
fn test_inject_creates_new_file() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();
    let result = injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    assert_eq!(result, InjectResult::Created);
    assert!(path.exists());
}

#[test]
fn test_inject_creates_file_with_markers() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();
    injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.contains("<!-- WAI:START -->"));
    assert!(content.contains("<!-- WAI:END -->"));
    assert!(content.contains("# Content"));
}

// ── BlockInjector: update existing block ───────────────────────────

#[test]
fn test_inject_updates_existing_block() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();

    // First injection
    injector
        .inject(&path, "WAI", "\n# Old content\n")
        .require("value");

    // Second injection — update
    let result = injector
        .inject(&path, "WAI", "\n# New content\n")
        .require("value");
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.contains("# New content"));
    assert!(!content.contains("# Old content"));
}

#[test]
fn test_inject_update_does_not_duplicate_markers() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();

    injector.inject(&path, "WAI", "content").require("value");
    injector.inject(&path, "WAI", "updated").require("value");

    let content = std::fs::read_to_string(&path).require("value");
    let start_count = content.matches("<!-- WAI:START -->").count();
    let end_count = content.matches("<!-- WAI:END -->").count();
    assert_eq!(start_count, 1, "should have exactly one WAI:START");
    assert_eq!(end_count, 1, "should have exactly one WAI:END");
}

// ── BlockInjector: prepend to existing file ────────────────────────

#[test]
fn test_inject_prepends_to_existing_file() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    std::fs::write(&path, "# Existing file\n").require("value");

    let injector = test_injector();
    let result = injector
        .inject(&path, "WAI", "\n# New block\n")
        .require("value");
    assert_eq!(result, InjectResult::Prepended);

    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.starts_with("<!-- WAI:START -->"));
    assert!(content.contains("# Existing file"));
}

// ── BlockInjector: has_block ───────────────────────────────────────

#[test]
fn test_has_block_false_when_file_missing() {
    let dir = tmp();
    let injector = test_injector();
    assert!(!injector.has_block(&dir.path().join("nonexistent.md"), "WAI"));
}

#[test]
fn test_has_block_false_when_no_block() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    std::fs::write(&path, "# No block\n").require("value");
    let injector = test_injector();
    assert!(!injector.has_block(&path, "WAI"));
}

#[test]
fn test_has_block_true_when_block_present() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();
    injector.inject(&path, "WAI", "content").require("value");
    assert!(injector.has_block(&path, "WAI"));
}

// ── BlockInjector: read_block ──────────────────────────────────────

#[test]
fn test_read_block_returns_none_when_missing() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    std::fs::write(&path, "# No block\n").require("value");
    let injector = test_injector();
    assert!(injector.read_block(&path, "WAI").is_none());
}

#[test]
fn test_read_block_returns_content() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();
    injector
        .inject(&path, "WAI", "\n# Block content\n")
        .require("value");
    let content = injector.read_block(&path, "WAI").require("value");
    assert!(content.contains("<!-- WAI:START -->"));
    assert!(content.contains("<!-- WAI:END -->"));
    assert!(content.contains("# Block content"));
}

// ── Custom markers ─────────────────────────────────────────────────

#[test]
fn test_custom_markers_work() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::with_markers(
        "ah:managed",
        "<!-- ah:managed:start -->",
        "<!-- ah:managed:end -->",
    ));
    let injector = BlockInjector::new(reg);

    let dir = tmp();
    let path = dir.path().join("test.md");
    injector
        .inject(&path, "ah:managed", "\n# ah content\n")
        .require("value");

    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.contains("<!-- ah:managed:start -->"));
    assert!(content.contains("<!-- ah:managed:end -->"));
}

// ── Error handling ─────────────────────────────────────────────────

#[test]
fn test_inject_unknown_block_returns_error() {
    let injector = BlockInjector::new(BlockRegistry::new());
    let dir = tmp();
    let path = dir.path().join("test.md");
    let result = injector.inject(&path, "UNKNOWN", "content");
    assert!(result.is_err());
}

#[test]
fn test_multiple_blocks_in_same_file() {
    let dir = tmp();
    let path = dir.path().join("test.md");
    let injector = test_injector();

    injector
        .inject(&path, "WAI", "\n# WAI content\n")
        .require("value");
    injector
        .inject(&path, "DONT", "\n# DONT content\n")
        .require("value");

    let content = std::fs::read_to_string(&path).require("value");
    assert!(content.contains("<!-- WAI:START -->"));
    assert!(content.contains("<!-- WAI:END -->"));
    assert!(content.contains("<!-- DONT:START -->"));
    assert!(content.contains("<!-- DONT:END -->"));
}

#[test]
fn test_reregister_replaces_old_block() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("WAI"));
    assert_eq!(reg.names().len(), 1);
}

// ── Provenance footer (add-artifact-provenance §1) ─────────────────

/// Extract the sha8 field from a provenance footer line.
fn footer_sha(line: &str) -> &str {
    let rest = line.split("sha=").nth(1).require("footer has sha=");
    rest.split(' ').next().require("sha terminated by space")
}

/// RED 1.1: footer-less injection is byte-identical to pre-footer output.
#[test]
fn test_footerless_injection_byte_identical_to_golden() {
    let dir = tmp();
    let path = dir.path().join("golden.md");
    let injector = test_injector();
    injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(content, "<!-- WAI:START -->\n# Content\n<!-- WAI:END -->");
    assert!(!content.contains("provenance:"));
}

/// RED 1.2: opt-in provenance footer written inside the markers.
#[test]
fn test_provenance_footer_written_inside_markers() {
    let dir = tmp();
    let path = dir.path().join("prov.md");
    let injector = BlockInjector::new(test_registry()).with_provenance("genesis");
    injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    let content = std::fs::read_to_string(&path).require("value");

    let footer = content
        .lines()
        .find(|l| l.contains("provenance:"))
        .require("footer line present");
    assert!(footer.starts_with("<!-- ") && footer.ends_with(" -->"));
    assert!(footer.contains("genesis"));
    assert!(footer.contains(env!("CARGO_PKG_VERSION")));
    assert!(footer.contains("source=WAI"));
    assert!(footer.contains(&format!("sha={}", content_sha8("\n# Content\n"))));

    // Footer lives inside the markers.
    let start = content.find("<!-- WAI:START -->").require("value");
    let end = content.find("<!-- WAI:END -->").require("value");
    let fpos = content.find("provenance:").require("value");
    assert!(start < fpos && fpos < end);
}

/// RED 1.3a: footer round-trips through re-inject with updated hash.
#[test]
fn test_footer_round_trip_updates_hash() {
    let dir = tmp();
    let path = dir.path().join("rt.md");
    let injector = BlockInjector::new(test_registry()).with_provenance("genesis");

    injector.inject(&path, "WAI", "\n# Old\n").require("value");
    let first = std::fs::read_to_string(&path).require("value");
    let first_sha = footer_sha(
        first
            .lines()
            .find(|l| l.contains("provenance:"))
            .require("value"),
    )
    .to_string();

    let result = injector.inject(&path, "WAI", "\n# New\n").require("value");
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).require("value");
    let second_sha = footer_sha(
        content
            .lines()
            .find(|l| l.contains("provenance:"))
            .require("value"),
    );
    assert_ne!(first_sha, second_sha);
    assert_eq!(second_sha, content_sha8("\n# New\n"));
    assert!(injector.has_block(&path, "WAI"));
    assert_eq!(content.matches("provenance:").count(), 1);
}

/// RED 1.3b: content hash excludes the footer line itself — version
/// changes do not move the hash for identical content.
#[test]
fn test_footer_hash_excludes_footer_line_and_version() {
    let c = "\n# Same content\n";
    let f_v1 = provenance_footer("genesis", "0.1.0", "WAI", c);
    let f_v2 = provenance_footer("genesis", "9.9.9", "WAI", c);
    assert_ne!(f_v1, f_v2, "footer text carries the version");
    assert_eq!(footer_sha(&f_v1), footer_sha(&f_v2));
    assert_eq!(footer_sha(&f_v1), content_sha8(c));
}
