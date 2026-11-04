//! Unit tests for `discovery` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget).

// pretender: role=test
#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

// ── read_manifest ──────────────────────────────────────────────────

#[test]
fn test_read_manifest_nonexistent() {
    let dir = tmp();
    assert!(
        !has_manifest(dir.path()),
        "fixture must start without a manifest"
    );
    assert!(read_manifest(dir.path()).is_none());
}

#[test]
fn test_read_manifest_valid() {
    let dir = tmp();
    let path = dir.path().join(MANIFEST_FILENAME);
    std::fs::create_dir_all(path.parent().require("value")).require("value");
    std::fs::write(
        &path,
        r#"[tools.wai]
description = "Workflow manager"
detector = { type = "directory", path = ".wai" }

[tools.dont]
description = "Conventions"
detector = { type = "directory", path = ".dont" }
"#,
    )
    .require("value");

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 2);
    assert!(manifest.tools.contains_key("wai"));
    assert!(manifest.tools.contains_key("dont"));
    assert_eq!(manifest.tools["wai"].detector.path, ".wai");
}

// ── scan ───────────────────────────────────────────────────────────

#[test]
fn test_scan_no_manifest() {
    let dir = tmp();
    let tools = scan(dir.path());
    assert!(tools.is_empty());
}

/// Detection matrix: directory/file markers, missing markers, and unknown
/// detector types (merged table-driven form of four near-identical tests).
#[test]
fn test_scan_detection_matrix() {
    let manifest = |tool: &str, desc: &str, dtype: &str, dpath: &str| -> String {
        format!(
            "[tools.{tool}]
description = \"{desc}\"
detector = {{ type = \"{dtype}\", path = \"{dpath}\" }}
"
        )
    };
    let cases = [
        (
            manifest("wai", "Workflow manager", "directory", ".wai"),
            Some((".wai", true)),
            "wai",
            true,
            "directory detector fires when the marker dir exists",
        ),
        (
            manifest("wai", "Workflow manager", "directory", ".wai"),
            None,
            "wai",
            false,
            "directory detector stays silent when the marker dir is missing",
        ),
        (
            manifest("testaruda", "Test harness", "file", "testaruda.toml"),
            Some(("testaruda.toml", false)),
            "testaruda",
            true,
            "file detector fires when the marker file exists",
        ),
        (
            manifest("foo", "Unknown detector", "magic", ".foo"),
            Some((".foo", false)),
            "foo",
            false,
            "unknown detector type is never detected",
        ),
    ];
    for (toml, marker, name, detected, why) in cases {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().require("manifest parent dir"))
            .require("manifest parent dir");
        std::fs::write(&path, toml).require("write manifest");
        match marker {
            Some((marker_path, true)) => {
                std::fs::create_dir(dir.path().join(marker_path)).require("create marker dir");
            }
            Some((marker_path, false)) => {
                std::fs::write(dir.path().join(marker_path), "").require("create marker file");
            }
            None => {}
        }

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1, "{why}");
        assert_eq!(tools[0].name, name, "{why}");
        assert_eq!(tools[0].detected, detected, "{why}");
    }
}

// ── register ──────────────────────────────────────────────────────

#[test]
fn test_register_creates_manifest() {
    let dir = tmp();
    register(dir.path(), "my-tool", "My tool", "directory", ".my-tool").require("value");

    let path = dir.path().join(MANIFEST_FILENAME);
    assert!(path.is_file(), "manifest should be created");

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools["my-tool"].description, "My tool");
    assert_eq!(manifest.tools["my-tool"].detector.path, ".my-tool");
}

#[test]
fn test_register_appends_to_existing() {
    let dir = tmp();
    register(dir.path(), "tool-a", "A", "directory", ".a").require("value");
    register(dir.path(), "tool-b", "B", "file", "b.toml").require("value");

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 2);
}

#[test]
fn test_register_updates_existing() {
    let dir = tmp();
    register(dir.path(), "tool", "Old description", "directory", ".old").require("value");
    register(dir.path(), "tool", "New description", "file", "new.toml").require("value");

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools["tool"].description, "New description");
    assert_eq!(manifest.tools["tool"].detector.path, "new.toml");
    assert_eq!(manifest.tools["tool"].detector.detector_type, "file");
}

// ── unregister ────────────────────────────────────────────────────

#[test]
fn test_unregister_removes_entry() {
    let dir = tmp();
    register(dir.path(), "tool-a", "A", "directory", ".a").require("value");
    register(dir.path(), "tool-b", "B", "directory", ".b").require("value");

    unregister(dir.path(), "tool-a").require("value");

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 1);
    assert!(manifest.tools.contains_key("tool-b"));
}

#[test]
fn test_unregister_removes_file_when_empty() {
    let dir = tmp();
    register(dir.path(), "tool", "Only tool", "directory", ".t").require("value");
    unregister(dir.path(), "tool").require("value");

    let path = dir.path().join(MANIFEST_FILENAME);
    assert!(!path.exists(), "manifest should be deleted when empty");
}

#[test]
fn test_unregister_nonexistent_is_noop() {
    let dir = tmp();
    // No manifest at all — should not error, and must not create one
    unregister(dir.path(), "ghost").require("unregister without manifest");
    assert!(
        !dir.path().join(MANIFEST_FILENAME).exists(),
        "unregistering a ghost tool must not create a manifest"
    );
}

// ── list_tools ────────────────────────────────────────────────────

#[test]
fn test_list_tools_empty() {
    let dir = tmp();
    assert!(list_tools(dir.path()).is_empty());
    assert!(
        !has_manifest(dir.path()),
        "no manifest means no tools to list"
    );
}

#[test]
fn test_list_tools_returns_names() {
    let dir = tmp();
    register(dir.path(), "alpha", "A", "directory", ".a").require("value");
    register(dir.path(), "beta", "B", "directory", ".b").require("value");

    let mut names = list_tools(dir.path());
    names.sort();
    assert_eq!(names, vec!["alpha", "beta"]);
}

// ── has_manifest ──────────────────────────────────────────────────

#[test]
fn test_has_manifest_false() {
    let dir = tmp();
    assert!(!has_manifest(dir.path()));
}

#[test]
fn test_has_manifest_true() {
    let dir = tmp();
    register(dir.path(), "t", "t", "directory", ".t").require("value");
    assert!(has_manifest(dir.path()));
}

// ── DetectedTool ──────────────────────────────────────────────────

#[test]
fn test_scan_multiple_tools() {
    let dir = tmp();
    let path = dir.path().join(MANIFEST_FILENAME);
    std::fs::create_dir_all(path.parent().require("value")).require("value");

    // Create marker dirs
    std::fs::create_dir(dir.path().join(".present")).require("value");

    std::fs::write(
        &path,
        r#"[tools.present]
detector = { type = "directory", path = ".present" }

[tools.absent]
detector = { type = "directory", path = ".absent" }
"#,
    )
    .require("value");

    let tools = scan(dir.path());
    assert_eq!(tools.len(), 2);

    let present = tools.iter().find(|t| t.name == "present").require("value");
    assert!(present.detected);

    let absent = tools.iter().find(|t| t.name == "absent").require("value");
    assert!(!absent.detected);
}
