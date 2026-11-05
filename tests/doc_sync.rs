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

mod common;

use common::TestUnwrap;

use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

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

fn project_file(rel: &str) -> PathBuf {
    Path::new(MANIFEST_DIR).join(rel)
}

fn read(rel: &str) -> String {
    fs::read_to_string(project_file(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
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
        .strip_prefix(MANIFEST_DIR)
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

/// Every `#[test]` fn in tests/doc_examples.rs must have a `→ docs/…` mapping
/// entry, and every mapping entry must point at a file that exists.
#[test]
fn snippet_map_covers_tests() {
    let source = read("tests/doc_examples.rs");

    let test_fns = parse_test_fns(&source);
    assert!(!test_fns.is_empty(), "no tests found in doc_examples.rs");

    let (mapped_fns, mapped_paths) = parse_snippet_map(&source);
    assert!(
        !mapped_fns.is_empty(),
        "no mapping entries found in doc_examples.rs header"
    );

    let problems = snippet_map_problems(&test_fns, &mapped_fns, &mapped_paths);
    assert!(
        problems.is_empty(),
        "snippet map out of sync:\n{}",
        problems.join("\n")
    );
}

/// `#[test]` fn names in a source file, sorted.
fn parse_test_fns(source: &str) -> Vec<String> {
    let mut test_fns = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for pair in lines.windows(2) {
        let name = attr_fn_name(pair[0].trim(), pair[1].trim());
        if let Some(name) = name {
            test_fns.push(name);
        }
    }
    test_fns.sort();
    test_fns
}

/// Name of a `#[test]` fn when `prev` is its attribute and `line` its signature.
fn attr_fn_name(prev: &str, line: &str) -> Option<String> {
    if prev != "#[test]" || !line.starts_with("fn ") {
        return None;
    }
    let name = line["fn ".len()..].split('(').next().unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Mapping entries from the `//!` header: `- `fn` → docs/<file>, <section>`.
fn parse_snippet_map(source: &str) -> (Vec<String>, Vec<String>) {
    let mut mapped_fns = Vec::new();
    let mut mapped_paths = Vec::new();
    for line in source.lines() {
        let Some((fn_name, path)) = parse_mapping_line(line) else {
            continue;
        };
        mapped_fns.push(fn_name);
        mapped_paths.push(path);
    }
    (mapped_fns, mapped_paths)
}

fn parse_mapping_line(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_start_matches("//!").trim();
    let rest = line.strip_prefix("- `")?;
    let (fn_name, target) = rest.split_once("` ")?;
    // target is "` → docs/<file>, <section>" — drop the closing backtick,
    // the arrow, and surrounding whitespace before extracting the path
    let target = target
        .trim()
        .trim_start_matches('`')
        .trim()
        .trim_start_matches("→")
        .trim();
    let path = target.split(',').next().unwrap_or("").trim().to_string();
    Some((fn_name.to_string(), path))
}

fn snippet_map_problems(
    test_fns: &[String],
    mapped_fns: &[String],
    mapped_paths: &[String],
) -> Vec<String> {
    let mut problems = Vec::new();
    problems.extend(mapping_coverage_problems(test_fns, mapped_fns));
    problems.extend(mapping_target_problems(mapped_paths));
    problems
}

/// Tests without a mapping entry, and entries for removed tests.
fn mapping_coverage_problems(test_fns: &[String], mapped_fns: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let missing: Vec<&String> = test_fns
        .iter()
        .filter(|f| !mapped_fns.contains(f))
        .collect();
    if !missing.is_empty() {
        problems.push(format!(
            "tests without a mapping entry: {missing:?} — add `//! - `<test>` → docs/<file>, <section>` to the header"
        ));
    }
    let stale: Vec<&String> = mapped_fns
        .iter()
        .filter(|f| !test_fns.contains(f))
        .collect();
    if !stale.is_empty() {
        problems.push(format!(
            "mapping entries for tests that no longer exist: {stale:?}"
        ));
    }
    problems
}

/// Mapping entries pointing at files that don't exist.
fn mapping_target_problems(mapped_paths: &[String]) -> Vec<String> {
    mapped_paths
        .iter()
        .filter(|p| !project_file(p).exists())
        .map(|p| format!("mapping points at missing file: {p}"))
        .collect()
}


/// Install pins in README and the book must track Cargo.toml's version.
#[test]
fn version_pins_match_manifest() {
    let manifest = read("Cargo.toml");
    let version_line = manifest
        .lines()
        .find(|l| l.starts_with("version = "))
        .require("Cargo.toml has a version");
    let full = version_line
        .split('"')
        .nth(1)
        .require("version is quoted, e.g. \"0.6.0\"");

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
        check_version_pin(path, content, &pin, full, &mut problems);
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
