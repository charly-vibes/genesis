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

use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// The human-maintained source of the forbidden-API blocklist. Patterns live
/// here as a markdown table so the removed-API history is readable (and
/// updatable) without touching this test. This file is excluded from the
/// pattern scan — it necessarily quotes the forbidden APIs.
const REMOVED_APIS_DOC: &str = "docs/explanation/removed-apis.md";

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

/// Markdown files on the onboarding path (docs/ recursively + README).
fn markdown_files() -> Vec<(String, String)> {
    let mut out = Vec::new();

    let docs = project_file("docs");
    fn visit(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).expect("read docs dir");
        for entry in entries {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            if name.as_deref() == Some("_book") {
                continue; // mdBook build output — may hold stale copies
            }
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|e| e == "md") {
                let rel = path
                    .strip_prefix(MANIFEST_DIR)
                    .expect("under manifest")
                    .to_string_lossy()
                    .into_owned();
                let content = fs::read_to_string(&path).expect("read md");
                out.push((rel, content));
            }
        }
    }
    if docs.is_dir() {
        visit(&docs, &mut out);
    }
    out.push(("README.md".to_string(), read("README.md")));
    out
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
        for (pattern, why) in &forbidden {
            if content.contains(pattern.as_str()) {
                violations.push(format!("{path}: contains {pattern:?} — {why}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "docs drifted from the API:\n{}",
        violations.join("\n")
    );
}

/// Every `#[test]` fn in tests/doc_examples.rs must have a `→ docs/…` mapping
/// entry, and every mapping entry must point at a file that exists.
#[test]
fn snippet_map_covers_tests() {
    let source = read("tests/doc_examples.rs");
    let lines: Vec<&str> = source.lines().collect();

    let mut test_fns: Vec<String> = Vec::new();
    for pair in lines.windows(2) {
        let (prev, line) = (pair[0].trim(), pair[1].trim());
        if prev == "#[test]" && line.starts_with("fn ") {
            let name = line["fn ".len()..]
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() {
                test_fns.push(name);
            }
        }
    }
    test_fns.sort();
    assert!(!test_fns.is_empty(), "no tests found in doc_examples.rs");

    let mut mapped_paths: Vec<String> = Vec::new();
    let mut mapped_fns: Vec<String> = Vec::new();
    for line in source.lines() {
        // mapping entries live in the `//!` header: `- `fn` → docs/<file>, <section>`
        let line = line.trim().trim_start_matches("//!").trim();
        let Some(rest) = line.strip_prefix("- `") else {
            continue;
        };
        let Some((fn_name, target)) = rest.split_once("` ") else {
            continue;
        };
        // target is "` → docs/<file>, <section>" — drop the closing backtick,
        // the arrow, and surrounding whitespace before extracting the path
        let target = target
            .trim()
            .trim_start_matches('`')
            .trim()
            .trim_start_matches("→")
            .trim();
        let path = target.split(',').next().unwrap_or("").trim().to_string();
        mapped_fns.push(fn_name.to_string());
        mapped_paths.push(path);
    }
    assert!(
        !mapped_fns.is_empty(),
        "no mapping entries found in doc_examples.rs header"
    );

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

    for path in &mapped_paths {
        if !project_file(path).exists() {
            problems.push(format!("mapping points at missing file: {path}"));
        }
    }

    assert!(
        problems.is_empty(),
        "snippet map out of sync:\n{}",
        problems.join("\n")
    );
}

/// Install pins in README and the book must track Cargo.toml's version.
#[test]
fn version_pins_match_manifest() {
    let manifest = read("Cargo.toml");
    let version_line = manifest
        .lines()
        .find(|l| l.starts_with("version = "))
        .expect("Cargo.toml has a version");
    let full = version_line
        .split('"')
        .nth(1)
        .expect("version is quoted, e.g. \"0.6.0\"");

    let major_minor = full
        .rsplit_once('.')
        .expect("version has at least major.minor.patch")
        .0;
    let pin = format!("genesis-vibes = \"{major_minor}\"");
    let tag_pin = format!("tag = \"v{full}\"");

    let readme = read("README.md");
    let getting_started = read("docs/getting-started.md");

    let mut problems = Vec::new();
    for (path, content) in [
        ("README.md", readme.as_str()),
        ("docs/getting-started.md", getting_started.as_str()),
    ] {
        if !content.contains(&pin) {
            problems.push(format!(
                "{path}: missing install pin `{pin}` (Cargo.toml is at {full})"
            ));
        }
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
