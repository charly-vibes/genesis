//! Contract tests for `genesis::git` changed/uncommitted-file enumeration.
//!
//! Spec: enumeration set definitions (staged vs HEAD, between two revs,
//! untracked asymmetry), unborn-HEAD fallback, and lossy degradation.

mod common;

use common::TestUnwrap;
use genesis::fixture::Fixture;
use genesis::git;

#[test]
fn changed_files_staged_only_vs_head() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("staged.txt"), "staged content").require("write file");
    let out = fixture
        .run(&["git", "add", "staged.txt"])
        .require("run git add");
    assert!(out.success(), "git add failed: {}", out.stderr);

    let files = git::changed_files(fixture.root()).require("changed_files");
    assert_eq!(
        files,
        vec!["staged.txt"],
        "staged change vs HEAD must be listed"
    );
}

#[test]
fn changed_files_between_two_revs() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("second.txt"), "second").require("write file");
    for args in [
        &(["git", "add", "second.txt"])[..],
        &(["git", "commit", "-m", "second"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    let files = git::changed_files_between(fixture.root(), "HEAD~1", "HEAD")
        .require("changed_files_between");
    assert_eq!(files, vec!["second.txt"]);
}

#[test]
fn changed_files_unborn_head_falls_back_to_untracked() {
    // Fresh repository: `git init` only, no commits — HEAD does not exist.
    let fixture = Fixture::new().build().require("fixture");
    let out = fixture.run(&["git", "init"]).require("run git init");
    assert!(out.success(), "git init failed: {}", out.stderr);
    std::fs::write(fixture.path("untracked.txt"), "content").require("write file");
    std::fs::create_dir_all(fixture.path("sub")).require("create subdir");
    std::fs::write(fixture.path("sub/more.txt"), "more").require("write nested file");

    let files = git::changed_files(fixture.root()).require("unborn HEAD must not be an error");
    assert!(
        files.contains(&"untracked.txt".to_string()) && files.contains(&"sub/more.txt".to_string()),
        "unborn HEAD must fall back to untracked enumeration, got: {files:?}"
    );
    assert_eq!(files.len(), 2, "fallback must not be an empty set");
}

#[test]
fn changed_files_exclude_untracked_with_head() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitkeep"), "modified").require("modify tracked file");
    std::fs::write(fixture.path("untracked.txt"), "content").require("write untracked file");

    let files = git::changed_files(fixture.root()).require("changed_files");
    assert!(
        files.contains(&".gitkeep".to_string()),
        "modified tracked file must be listed, got: {files:?}"
    );
    assert!(
        !files.contains(&"untracked.txt".to_string()),
        "untracked files must be excluded while HEAD resolves, got: {files:?}"
    );
}

#[test]
fn uncommitted_files_include_untracked() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitkeep"), "modified").require("modify tracked file");
    std::fs::write(fixture.path("untracked.txt"), "content").require("write untracked file");

    let files = git::uncommitted_files(fixture.root()).require("uncommitted_files");
    assert!(
        files.contains(&".gitkeep".to_string()) && files.contains(&"untracked.txt".to_string()),
        "uncommitted files must include tracked modifications and untracked entries, got: {files:?}"
    );
}

#[test]
fn uncommitted_files_clean_repo_is_empty() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    let files = git::uncommitted_files(fixture.root()).require("uncommitted_files");
    assert!(
        files.is_empty(),
        "clean tree must enumerate to empty, got: {files:?}"
    );
}

#[test]
fn uncommitted_files_staged_rename_reports_new_path() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("old.txt"), "content").require("write file");
    for args in [
        &(["git", "add", "old.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
        &(["git", "mv", "old.txt", "new.txt"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    let files = git::uncommitted_files(fixture.root()).require("uncommitted_files");
    assert_eq!(
        files,
        vec!["new.txt"],
        "staged rename must report the new path only"
    );
}

/// `core.quotePath` makes `git diff --name-only` emit C-style-quoted
/// paths for non-ASCII names; enumeration must unquote them so
/// changed_files and uncommitted_files report identical strings.
#[test]
fn quoted_non_ascii_paths_match_uncommitted_files() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    seed_commit_and_stage_non_ascii(&fixture);

    let changed = git::changed_files(fixture.root()).require("changed_files");
    let uncommitted = git::uncommitted_files(fixture.root()).require("uncommitted_files");
    assert_eq!(
        changed, uncommitted,
        "quoted ls/diff output must unquote identically to porcelain parsing"
    );
    assert_eq!(
        changed,
        vec!["café.txt".to_string()],
        "non-ASCII name must round-trip unquoted, got: {changed:?}"
    );
}

/// Seed the fixture repo: a first commit (tracked-change enumeration
/// needs a HEAD to diff against), then the non-ASCII file staged so
/// `core.quotePath` applies to it.
///
/// changed_files enumerates tracked changes against HEAD, so an untracked
/// file is invisible to that list; the parity contract is that both
/// enumerators unquote a quoted `core.quotePath` name identically.
fn seed_commit_and_stage_non_ascii(fixture: &Fixture) {
    std::fs::write(fixture.path(".gitkeep"), "seed").require("write file");
    for args in [
        &(["git", "add", ".gitkeep"])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }
    std::fs::write(fixture.path("café.txt"), "content").require("write non-ascii file");
    let out = fixture.run(&["git", "add", "café.txt"]).require("run git");
    assert!(out.success(), "add failed: {}", out.stderr);
}

#[test]
fn lossy_variant_returns_empty_on_failure() {
    // A directory with no repository at all: the non-lossy variant errors,
    // the lossy variant degrades to an empty collection by contract.
    let fixture = Fixture::new().build().require("fixture without git");
    assert!(git::uncommitted_files(fixture.root()).is_err());
    assert!(
        git::uncommitted_files_lossy(fixture.root()).is_empty(),
        "lossy variant must return an empty set on git failure"
    );
}

#[test]
fn changed_files_lossy_returns_empty_on_failure() {
    // Symmetry with `uncommitted_files_lossy` (tpf.3 finding): the non-lossy
    // variant errors, the lossy variant degrades to an empty collection.
    let fixture = Fixture::new().build().require("fixture without git");
    assert!(git::changed_files(fixture.root()).is_err());
    assert!(
        git::changed_files_lossy(fixture.root()).is_empty(),
        "lossy variant must return an empty set on git failure"
    );
}

#[test]
fn changed_files_lossy_matches_changed_files_on_success() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("staged.txt"), "staged content").require("write file");
    let out = fixture
        .run(&["git", "add", "staged.txt"])
        .require("run git add");
    assert!(out.success(), "git add failed: {}", out.stderr);

    assert_eq!(
        git::changed_files_lossy(fixture.root()),
        git::changed_files(fixture.root()).require("changed_files"),
        "lossy variant must match the erroring variant when git succeeds"
    );
}
