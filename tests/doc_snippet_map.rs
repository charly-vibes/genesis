//! Snippet-map traceability guard (third of the doc_sync drift guards,
//! genesis-2x9): every `#[test]` in `tests/doc_examples.rs` must have a
//! `→ docs/…` mapping entry in its `//!` header pointing at a real doc
//! file, so mirrors can't be added or renamed without traceability.

mod common;

use common::{project_file, read};

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

/// Parse one mapping line, extracting the mapped fn name and target doc
/// path: after ``- `fn` ``, the target is `` ` → docs/<file>, <section> ``
/// — drop the closing backtick, the arrow, and surrounding whitespace
/// before splitting off the path.
fn parse_mapping_line(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_start_matches("//!").trim();
    let rest = line.strip_prefix("- `")?;
    let (fn_name, target) = rest.split_once("` ")?;
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
