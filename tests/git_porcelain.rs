//! Contract tests for the porcelain v1 parsing contract of
//! `genesis::git` (pure unit tests over `parse_porcelain`).
//!
//! Spec: plain `XY path` records, rename/copy `XY old -> new` records
//! (reporting `new` only), quoted paths (`core.quotePath` C-style
//! escapes), non-ASCII names, and empty status bodies.

mod common;
use genesis::git::parse_porcelain;

#[test]
fn porcelain_rename_reports_new_path() {
    let parsed = parse_porcelain("R  old-name.rs -> new-name.rs\n");
    assert_eq!(
        parsed,
        vec!["new-name.rs"],
        "rename must report the new path"
    );
    assert!(
        !parsed.iter().any(|p| p.contains(" -> ")),
        "the joined 'old -> new' string must never appear: {parsed:?}"
    );

    // Rename with an additional worktree marker (XY = `RM`).
    let parsed = parse_porcelain("RM staged.old -> staged.new\n");
    assert_eq!(parsed, vec!["staged.new"]);
}

#[test]
fn porcelain_copy_records_report_new_path() {
    let parsed = parse_porcelain("C  origin.txt -> copy.txt\n");
    assert_eq!(
        parsed,
        vec!["copy.txt"],
        "copy records must report the new path"
    );

    // Copy with an additional worktree marker (XY = `CM`).
    let parsed = parse_porcelain("CM a.tpl -> b.tpl\n");
    assert_eq!(parsed, vec!["b.tpl"]);
}

#[test]
fn porcelain_quoted_paths_are_unquoted() {
    // `core.quotePath` escapes non-ASCII as octal UTF-8 bytes: é = \303\251.
    let parsed = parse_porcelain("M  \"caf\\303\\251.txt\"\n");
    assert_eq!(parsed, vec!["café.txt"], "octal escapes must be unquoted");

    // C-style escapes for special characters: tab, quote, backslash.
    let parsed = parse_porcelain("?? \"tab\\there\"\n");
    assert_eq!(parsed, vec!["tab\there"]);

    let parsed = parse_porcelain("M  \"quo\\\"ted\\\\path\"\n");
    assert_eq!(parsed, vec!["quo\"ted\\path"]);

    // Quoted paths inside subdirectories keep the directory prefix.
    let parsed = parse_porcelain("A  \"sub/dir\\303\\251.txt\"\n");
    assert_eq!(parsed, vec!["sub/diré.txt"]);
}

#[test]
fn porcelain_non_ascii_names_pass_through_unquoted() {
    // With core.quotePath=false, git emits raw UTF-8 paths.
    let parsed = parse_porcelain("M  überDatei.md\n");
    assert_eq!(parsed, vec!["überDatei.md"]);
}

#[test]
fn porcelain_plain_status_records_pass_through() {
    let parsed = parse_porcelain("M  modified.rs\n M unstaged.rs\n?? untracked.txt\nA  added.rs\n");
    assert_eq!(
        parsed,
        vec!["modified.rs", "unstaged.rs", "untracked.txt", "added.rs"]
    );
}

#[test]
fn porcelain_empty_status_is_empty_ok() {
    assert!(
        parse_porcelain("").is_empty(),
        "empty body must parse to empty"
    );
    assert!(
        parse_porcelain("\n").is_empty(),
        "newline-only body must parse to empty"
    );
}
