//! Unit tests for `discovery` (split out to keep the app file under the file-lines budget).

// pretender: role=test
use super::*;

use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

/// Write `toml` as the project manifest inside `dir`, creating `.genesis/`.
fn seed_manifest(dir: &TempDir, toml: &str) {
    let path = dir.path().join(MANIFEST_FILENAME);
    std::fs::create_dir_all(path.parent().require("manifest parent dir"))
        .require("manifest parent dir");
    std::fs::write(&path, toml).require("write manifest");
}

/// Create a detector marker on disk: a directory when `is_dir`, else a file.
fn seed_marker(dir: &TempDir, marker_path: &str, is_dir: bool) {
    let target = dir.path().join(marker_path);
    if is_dir {
        std::fs::create_dir(target).require("create marker dir");
    } else {
        std::fs::write(target, "").require("create marker file");
    }
}

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
    seed_manifest(&dir, THREE_TOOLS);

    let manifest = read_manifest(dir.path()).require("value");
    assert_eq!(manifest.tools.len(), 3);
    assert!(manifest.tools.contains_key("wai"));
    assert!(manifest.tools.contains_key("dont"));
    assert_eq!(manifest.tools["wai"].detector.path, ".wai");
}

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
            manifest("wai", "w", "directory", ".wai"),
            Some((".wai", true)),
            "wai",
            true,
        ),
        (
            manifest("wai", "w", "directory", ".wai"),
            None,
            "wai",
            false,
        ),
        (
            manifest("t", "t", "file", "testaruda.toml"),
            Some(("testaruda.toml", false)),
            "t",
            true,
        ),
        (
            manifest("foo", "f", "magic", ".foo"),
            Some((".foo", false)),
            "foo",
            false,
        ),
    ];
    for (i, (toml, marker, name, detected)) in cases.iter().enumerate() {
        let dir = tmp();
        seed_manifest(&dir, toml);
        if let Some((marker_path, is_dir)) = *marker {
            seed_marker(&dir, marker_path, is_dir);
        }

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1, "case {i}");
        assert_eq!(tools[0].name, *name, "case {i}");
        assert_eq!(tools[0].detected, *detected, "case {i}");
    }
}

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
    unregister(dir.path(), "ghost").require("unregister without manifest");
    assert!(
        !dir.path().join(MANIFEST_FILENAME).exists(),
        "ghost tool must not create manifest"
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
    seed_marker(&dir, ".present", true);
    seed_manifest(
        &dir,
        r#"[tools.present]
detector = { type = "directory", path = ".present" }

[tools.absent]
detector = { type = "directory", path = ".absent" }
"#,
    );

    let tools = scan(dir.path());
    assert_eq!(tools.len(), 2);

    let present = tools.iter().find(|t| t.name == "present").require("value");
    assert!(present.detected);

    let absent = tools.iter().find(|t| t.name == "absent").require("value");
    assert!(!absent.detected);
}

// ── byte-stability ────────────────────────────────────────────────

/// 3-tool manifest in hand-written (unsorted) order. Descriptions omitted:
/// ordering is what under test.
const THREE_TOOLS: &str = r#"[tools.wai]
detector = { type = "directory", path = ".wai" }

[tools.dont]
detector = { type = "directory", path = ".dont" }

[tools.kaya]
detector = { type = "directory", path = ".kaya" }
"#;

/// One read+write cycle; returns the bytes on disk after it.
fn rewrite_cycle(dir: &TempDir) -> Vec<u8> {
    let path = dir.path().join(MANIFEST_FILENAME);
    let manifest = read_manifest(dir.path()).require("read manifest");
    write_manifest(&path, &manifest).require("rewrite manifest");
    std::fs::read(&path).require("read manifest back")
}

/// The `[tools.*]` table names, in serialized order.
fn tool_table_order(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|l| l.strip_prefix("[tools.")?.strip_suffix(']'))
        .filter(|name| !name.contains('.'))
        .map(str::to_owned)
        .collect()
}

/// Rewrite round-trips must be byte-identical: canonical order is fully
/// sorted tool names (BTreeMap), so cycles never churn. The first rewrite
/// may normalize hand-edited formatting; from then on bytes are stable.
#[test]
fn test_manifest_rewrite_is_byte_stable() {
    let dir = tmp();
    seed_manifest(&dir, THREE_TOOLS);

    rewrite_cycle(&dir); // normalization pass
    let second = rewrite_cycle(&dir);
    let third = rewrite_cycle(&dir);

    assert_eq!(second, third, "rewrite cycle changed the file bytes");
    assert_eq!(
        tool_table_order(&third),
        ["dont", "kaya", "wai"],
        "canonical sorted order"
    );
}
