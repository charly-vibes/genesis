//! Drift guards between the mdBook docs and the code they document.
//!
//! genesis-2x9: `tests/doc_examples.rs` mirrors mdBook snippets by hand; this
//! file is the enforcement layer that makes drift fail `cargo test`:
//!
//! 1. `forbidden_api_patterns` — removed/renamed API usages must not reappear
//!    in any markdown doc (the drift that broke the onboarding path between
//!    v0.4 and v0.6). The blocklist is *derived from* the human-maintained
//!    anchor doc [`REMOVED_APIS_DOC`] (genesis-h9k) — add a row there, not here.
//! 2. `snippet_map_covers_tests` — every test in `tests/doc_examples.rs` must
//!    have a mapping entry pointing at a real doc file, so mirrors can't be
//!    added or renamed without traceability.
//! 3. `version_pins_match_manifest` — install pins in README and the book
//!    must track `Cargo.toml`'s version.
//!
//! `snippet_map_covers_tests`, the third drift guard, lives in
//! `tests/doc_snippet_map.rs`.

mod common;

use common::{TestUnwrap, project_file, read};

use std::fs;
use std::path::Path;

/// The human-maintained source of the forbidden-API blocklist. Patterns live
/// here as a markdown table so the removed-API history is readable (and
/// updatable) without touching this test. This file is excluded from the
/// pattern scan — it necessarily quotes the forbidden APIs.
const REMOVED_APIS_DOC: &str = "docs/src/explanation/removed-apis.md";

/// Parse the `| `pattern` | reason |` rows out of the anchor doc.
fn load_forbidden_patterns() -> Vec<(String, String)> {
    let content = fs::read_to_string(project_file(REMOVED_APIS_DOC)).unwrap_or_else(|e| {
        panic!(
            "forbidden-API anchor {REMOVED_APIS_DOC} is missing ({e}) — \
                 it is the maintained source of this guard's blocklist"
        )
    });

    let mut patterns = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        // data rows look like: | `pattern` | reason |  (header has no backtick)
        if !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert!(
            cells.len() == 2,
            "{REMOVED_APIS_DOC} table row must be exactly `| pattern | reason |`: {line}"
        );
        let pattern = cells[0]
            .strip_prefix('`')
            .and_then(|p| p.strip_suffix('`'))
            .unwrap_or_else(|| {
                panic!("{REMOVED_APIS_DOC}: pattern must be backtick-quoted: {line}")
            });
        let reason = cells[1];
        assert!(
            !reason.is_empty(),
            "{REMOVED_APIS_DOC}: empty reason: {line}"
        );
        patterns.push((pattern.to_string(), reason.to_string()));
    }
    patterns
}

/// Markdown files on the onboarding path (the mdBook source under docs/src,
/// recursively) + README.
fn markdown_files() -> Vec<(String, String)> {
    let mut out = Vec::new();

    let docs = project_file("docs/src");
    if docs.is_dir() {
        visit_docs(&docs, &mut out);
    }
    out.push(("README.md".to_string(), read("README.md")));
    out
}

/// Recursively collect markdown files under `dir`.
fn visit_docs(dir: &Path, out: &mut Vec<(String, String)>) {
    let entries = fs::read_dir(dir).require("read docs dir");
    let paths = entries
        .map(|e| e.require("dir entry").path())
        .filter(|p| !is_skipped_dir(p)); // mdBook output / spec copies may hold stale content
    for path in paths {
        doc_entry(&path, out);
    }
}

/// mdBook build output / deployed spec copies — may hold stale content.
fn is_skipped_dir(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    matches!(name.as_deref(), Some("_book") | Some("specs"))
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md")
}

/// One walk entry: recurse into dirs, collect markdown files.
fn doc_entry(path: &Path, out: &mut Vec<(String, String)>) {
    if path.is_dir() {
        return visit_docs(path, out);
    }
    if is_markdown(path) {
        out.push(read_doc_file(path));
    }
}

fn read_doc_file(path: &Path) -> (String, String) {
    let rel = path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .require("under manifest")
        .to_string_lossy()
        .into_owned();
    let content = fs::read_to_string(path).require("read md");
    (rel, content)
}

/// APIs removed or renamed in the past must not resurface in docs.
///
/// The blocklist comes from [`REMOVED_APIS_DOC`] — the anchor doc is the
/// maintained source (genesis-h9k). Add rows there, not here.
#[test]
fn forbidden_api_patterns() {
    let forbidden = load_forbidden_patterns();

    // The anchor doc necessarily quotes the forbidden patterns; scan everything else.
    let files: Vec<(String, String)> = markdown_files()
        .into_iter()
        .filter(|(path, _)| path != REMOVED_APIS_DOC)
        .collect();
    assert!(!files.is_empty(), "expected markdown docs to scan");

    let mut violations = Vec::new();
    for (path, content) in &files {
        violations.extend(forbidden_hits(path, content, &forbidden));
    }

    assert!(
        violations.is_empty(),
        "docs drifted from the API:\n{}",
        violations.join("\n")
    );
}

/// All blocklist hits in one markdown file.
fn forbidden_hits(path: &str, content: &str, forbidden: &[(String, String)]) -> Vec<String> {
    let mut violations = Vec::new();
    for (pattern, why) in forbidden {
        if content.contains(pattern.as_str()) {
            violations.push(format!("{path}: contains {pattern:?} — {why}"));
        }
    }
    violations
}

/// `version` value of the `[package]` section in Cargo.toml.
fn manifest_version() -> String {
    let manifest = read("Cargo.toml");
    let version_line = manifest
        .lines()
        .find(|l| l.starts_with("version = "))
        .require("Cargo.toml has a version");
    version_line
        .split('"')
        .nth(1)
        .require("version is quoted, e.g. \"0.6.0\"")
        .to_string()
}

/// Install pins in README and the book must track Cargo.toml's version.
#[test]
fn version_pins_match_manifest() {
    let full = manifest_version();
    let major_minor = full
        .rsplit_once('.')
        .require("version has at least major.minor.patch")
        .0;
    let pin = format!("genesis-vibes = \"{major_minor}\"");
    let tag_pin = format!("tag = \"v{full}\"");

    let readme = read("README.md");
    let getting_started = read("docs/src/getting-started.md");

    let mut problems = Vec::new();
    for (path, content) in [
        ("README.md", readme.as_str()),
        ("docs/src/getting-started.md", getting_started.as_str()),
    ] {
        check_version_pin(path, content, &pin, &full, &mut problems);
    }
    // at least one of README/getting-started documents the git-tag variant
    if !readme.contains(&tag_pin) && !getting_started.contains(&tag_pin) {
        problems.push(format!(
            "no git-tag pin `{tag_pin}` in README.md or docs/getting-started.md"
        ));
    }

    assert!(
        problems.is_empty(),
        "version pins drifted from the manifest:\n{}",
        problems.join("\n")
    );
}

fn check_version_pin(path: &str, content: &str, pin: &str, full: &str, problems: &mut Vec<String>) {
    if !content.contains(pin) {
        problems.push(format!(
            "{path}: missing install pin `{pin}` (Cargo.toml is at {full})"
        ));
    }
}
