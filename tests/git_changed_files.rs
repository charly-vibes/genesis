//! Contract tests for `genesis::git` changed/uncommitted-file enumeration.
//!
//! Spec: enumeration set definitions (staged vs HEAD, between two revs,
//! untracked asymmetry), unborn-HEAD fallback, and lossy degradation.

use genesis::fixture::Fixture;
use genesis::git;

#[test]
fn changed_files_staged_only_vs_head() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    std::fs::write(fixture.path("staged.txt"), "staged content").expect("write file");
    let out = fixture
        .run(&["git", "add", "staged.txt"])
        .expect("run git add");
    assert!(out.success(), "git add failed: {}", out.stderr);

    let files = git::changed_files(fixture.root()).expect("changed_files");
    assert_eq!(
        files,
        vec!["staged.txt"],
        "staged change vs HEAD must be listed"
    );
}

#[test]
fn changed_files_between_two_revs() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    std::fs::write(fixture.path("second.txt"), "second").expect("write file");
    for args in [
        &(["git", "add", "second.txt"])[..],
        &(["git", "commit", "-m", "second"])[..],
    ] {
        let out = fixture.run(args).expect("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    let files = git::changed_files_between(fixture.root(), "HEAD~1", "HEAD")
        .expect("changed_files_between");
    assert_eq!(files, vec!["second.txt"]);
}

#[test]
fn changed_files_unborn_head_falls_back_to_untracked() {
    // Fresh repository: `git init` only, no commits — HEAD does not exist.
    let fixture = Fixture::new().build().expect("fixture");
    let out = fixture.run(&["git", "init"]).expect("run git init");
    assert!(out.success(), "git init failed: {}", out.stderr);
    std::fs::write(fixture.path("untracked.txt"), "content").expect("write file");
    std::fs::create_dir_all(fixture.path("sub")).expect("create subdir");
    std::fs::write(fixture.path("sub/more.txt"), "more").expect("write nested file");

    let files = git::changed_files(fixture.root()).expect("unborn HEAD must not be an error");
    assert!(
        files.contains(&"untracked.txt".to_string()) && files.contains(&"sub/more.txt".to_string()),
        "unborn HEAD must fall back to untracked enumeration, got: {files:?}"
    );
    assert_eq!(files.len(), 2, "fallback must not be an empty set");
}

#[test]
fn changed_files_exclude_untracked_with_head() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    std::fs::write(fixture.path(".gitkeep"), "modified").expect("modify tracked file");
    std::fs::write(fixture.path("untracked.txt"), "content").expect("write untracked file");

    let files = git::changed_files(fixture.root()).expect("changed_files");
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
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    std::fs::write(fixture.path(".gitkeep"), "modified").expect("modify tracked file");
    std::fs::write(fixture.path("untracked.txt"), "content").expect("write untracked file");

    let files = git::uncommitted_files(fixture.root()).expect("uncommitted_files");
    assert!(
        files.contains(&".gitkeep".to_string()) && files.contains(&"untracked.txt".to_string()),
        "uncommitted files must include tracked modifications and untracked entries, got: {files:?}"
    );
}

#[test]
fn uncommitted_files_clean_repo_is_empty() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let files = git::uncommitted_files(fixture.root()).expect("uncommitted_files");
    assert!(
        files.is_empty(),
        "clean tree must enumerate to empty, got: {files:?}"
    );
}

#[test]
fn uncommitted_files_staged_rename_reports_new_path() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    std::fs::write(fixture.path("old.txt"), "content").expect("write file");
    for args in [
        &(["git", "add", "old.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
        &(["git", "mv", "old.txt", "new.txt"])[..],
    ] {
        let out = fixture.run(args).expect("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    let files = git::uncommitted_files(fixture.root()).expect("uncommitted_files");
    assert_eq!(
        files,
        vec!["new.txt"],
        "staged rename must report the new path only"
    );
}

#[test]
fn lossy_variant_returns_empty_on_failure() {
    // A directory with no repository at all: the non-lossy variant errors,
    // the lossy variant degrades to an empty collection by contract.
    let fixture = Fixture::new().build().expect("fixture without git");
    assert!(git::uncommitted_files(fixture.root()).is_err());
    assert!(
        git::uncommitted_files_lossy(fixture.root()).is_empty(),
        "lossy variant must return an empty set on git failure"
    );
}
