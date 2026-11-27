//! Read-only block inspection tests for `BlockInjector`: `has_block`
//! and `read_block` over missing, block-less, and wired files.

// pretender: role=test
use super::*;
use crate::testutil::TestUnwrap;

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
