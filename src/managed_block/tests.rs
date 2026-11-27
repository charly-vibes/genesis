//! Unit tests for `managed_block` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget). Provenance
//! footer tests live in the [`provenance`] submodule.

// pretender: role=test
#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use tempfile::TempDir;

mod inspect;
mod provenance;
mod splice;

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

/// Two blocks must not share a start marker — the injector would match
/// either block's region in the other's files (silent cross-block capture).
#[test]
#[should_panic(expected = "shares a marker with an already-registered block")]
fn test_registry_rejects_shared_start_marker() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    assert!(reg.has("WAI"), "first block must register cleanly");
    reg.register(BlockDef::with_markers(
        "OTHER",
        "<!-- WAI:START -->",
        "<!-- OTHER:END -->",
    ));
}

/// Sharing an end marker is rejected the same way.
#[test]
#[should_panic(expected = "shares a marker with an already-registered block")]
fn test_registry_rejects_shared_end_marker() {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    assert!(reg.has("WAI"), "first block must register cleanly");
    reg.register(BlockDef::with_markers(
        "OTHER",
        "<!-- OTHER:START -->",
        "<!-- WAI:END -->",
    ));
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
