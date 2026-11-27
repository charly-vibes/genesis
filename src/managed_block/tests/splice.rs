//! Splicing-behavior tests for `BlockInjector`: in-place update,
//! prepend-to-existing-file, and the quoted-end-marker guard.

// pretender: role=test
use super::*;
use crate::testutil::TestUnwrap;

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

/// A file that merely *quotes* the end marker before the real block must
/// not get the quoted prose spliced away: the end marker is searched only
/// after the start marker (mirroring `read_block`'s guard).
#[test]
fn test_inject_end_marker_before_start_is_not_corrupted() {
    let dir = tmp();
    let path = dir.path().join("quoted.md");
    std::fs::write(
        &path,
        "prose quoting the end marker: <!-- WAI:END -->\n\n<!-- WAI:START -->\n# Real content\n<!-- WAI:END -->\n",
    )
    .require("value");

    let injector = test_injector();
    let result = injector
        .inject(&path, "WAI", "\n# New content\n")
        .require("value");
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(
        content,
        "prose quoting the end marker: <!-- WAI:END -->\n\n<!-- WAI:START -->\n# New content\n<!-- WAI:END -->\n",
        "quoted prose before the block must survive the splice"
    );
    assert_eq!(content.matches("<!-- WAI:START -->").count(), 1);
    assert_eq!(content.matches("<!-- WAI:END -->").count(), 2);
    assert!(!content.contains("# Real content"));
}
