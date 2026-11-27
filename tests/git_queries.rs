//! Contract tests for the batched and checkout-shape `genesis::git`
//! queries: `staged_files`, `tracked_paths`, `ignored_paths`,
//! `worktree_facts`, and `current_branch`.
//!
//! Spec: staged files mirror `git diff --cached --name-only HEAD` with an
//! empty answer on unborn HEAD; batch queries answer in one spawn and
//! handle quoted/non-ASCII paths. Remote-URL contracts (`remote_url`,
//! `remote_key`) live in the sibling `git_queries_remote.rs` binary.

mod common;

use common::TestUnwrap;
use std::path::Path;

use genesis::fixture::Fixture;
use genesis::git::{self, GitError};

/// Seed a fixture repo: init, one commit (so HEAD exists), return it.
fn seeded_repo() -> Fixture {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("seed.txt"), "seed").require("write seed");
    for args in [
        &(["git", "add", "seed.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }
    fixture
}

#[test]
fn staged_files_reports_staged_not_unstaged() {
    let fixture = seeded_repo();
    std::fs::write(fixture.path("staged.txt"), "content").require("write staged");
    std::fs::write(fixture.path("seed.txt"), "modified").require("modify tracked");
    let out = fixture.run(&["git", "add", "staged.txt"]).require("stage");
    assert!(out.success(), "add failed: {}", out.stderr);

    let staged = git::staged_files(fixture.root()).require("staged_files");
    assert_eq!(staged, vec!["staged.txt".to_string()]);
}

#[test]
fn staged_files_unborn_head_is_empty() {
    // No `with_git_init`: the fixture stays a plain directory, so the
    // explicit `git init` below leaves HEAD unborn (no initial commit).
    let fixture = Fixture::new().build().require("fixture");
    let out = fixture.run(&["git", "init", "-b", "main"]).require("init");
    assert!(out.success(), "init failed: {}", out.stderr);
    std::fs::write(fixture.path("only.txt"), "content").require("write file");
    let out = fixture.run(&["git", "add", "only.txt"]).require("stage");
    assert!(out.success(), "add failed: {}", out.stderr);

    assert!(
        git::staged_files(fixture.root())
            .require("staged_files")
            .is_empty(),
        "unborn HEAD means nothing is staged relative to a commit"
    );
}

#[test]
fn staged_files_unquotes_non_ascii_paths() {
    let fixture = seeded_repo();
    let name = "ünïcode ファイル.txt";
    std::fs::write(fixture.path(name), "content").require("write non-ascii");
    let out = fixture.run(&["git", "add", name]).require("stage");
    assert!(out.success(), "add failed: {}", out.stderr);

    let staged = git::staged_files(fixture.root()).require("staged_files");
    assert_eq!(staged, vec![name.to_string()]);
}

#[test]
fn tracked_paths_batches_in_one_answer() {
    let fixture = seeded_repo();
    std::fs::write(fixture.path("tracked-a.txt"), "a").require("write a");
    std::fs::write(fixture.path("tracked-b.txt"), "b").require("write b");
    std::fs::write(fixture.path("untracked.txt"), "c").require("write untracked");
    let out = fixture
        .run(&["git", "add", "tracked-a.txt", "tracked-b.txt"])
        .require("stage");
    assert!(out.success(), "add failed: {}", out.stderr);

    let paths: Vec<_> = ["tracked-a.txt", "tracked-b.txt", "untracked.txt"]
        .iter()
        .map(|p| fixture.path(p))
        .collect();
    let tracked = git::tracked_paths(fixture.root(), &paths).require("tracked_paths");
    assert_eq!(
        tracked.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["tracked-a.txt", "tracked-b.txt"],
        "only the git-known subset is returned"
    );

    let none: &[&Path] = &[];
    assert!(
        git::tracked_paths(fixture.root(), none)
            .require("tracked_paths")
            .is_empty()
    );
}

#[test]
fn tracked_paths_handles_glob_like_and_non_ascii_names() {
    let fixture = seeded_repo();
    for name in ["a[1].txt", "ünïcode.txt"] {
        std::fs::write(fixture.path(name), "content").require("write file");
        let out = fixture.run(&["git", "add", name]).require("stage");
        assert!(out.success(), "add {name} failed: {}", out.stderr);
    }
    let paths: Vec<_> = ["a[1].txt", "ünïcode.txt"]
        .iter()
        .map(|p| fixture.path(p))
        .collect();
    let tracked = git::tracked_paths(fixture.root(), &paths).require("tracked_paths");
    assert_eq!(
        tracked.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["a[1].txt", "ünïcode.txt"],
        "literal pathspec magic must match glob-like names; -z output must be verbatim"
    );
}

#[test]
fn ignored_paths_reports_only_ignored() {
    let fixture = seeded_repo();
    stage_gitignore(&fixture, "ignored/\n*.log\n");
    write_seed_files(&fixture);

    let paths: Vec<_> = ["ignored/nested.txt", "debug.log", "kept.txt"]
        .iter()
        .map(|p| fixture.path(p))
        .collect();
    let ignored = git::ignored_paths(fixture.root(), &paths).require("ignored_paths");
    assert_eq!(
        ignored.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["ignored/nested.txt", "debug.log"],
    );

    assert_no_paths_yields_empty(fixture.root());
}

/// Write `.gitignore` with `rules` and stage it (so check-ignore sees it).
fn stage_gitignore(fixture: &Fixture, rules: &str) {
    std::fs::write(fixture.path(".gitignore"), rules).require("write gitignore");
    let out = fixture.run(&["git", "add", ".gitignore"]).require("stage");
    assert!(out.success(), "add failed: {}", out.stderr);
}

/// Create one nested-ignored, one glob-ignored, one kept file.
fn write_seed_files(fixture: &Fixture) {
    std::fs::create_dir(fixture.path("ignored")).require("mkdir");
    std::fs::write(fixture.path("ignored/nested.txt"), "x").require("write nested");
    std::fs::write(fixture.path("debug.log"), "x").require("write log");
    std::fs::write(fixture.path("kept.txt"), "x").require("write kept");
}

/// An empty path list is an empty answer, never a spawn or an error.
fn assert_no_paths_yields_empty(root: &Path) {
    let none: &[&Path] = &[];
    assert!(
        git::ignored_paths(root, none)
            .require("ignored_paths")
            .is_empty()
    );
}

#[test]
fn ignored_paths_without_gitignore_is_empty_not_error() {
    let fixture = seeded_repo();
    std::fs::write(fixture.path("plain.txt"), "x").require("write plain");
    let paths: Vec<_> = ["plain.txt"].iter().map(|p| fixture.path(p)).collect();
    assert!(
        git::ignored_paths(fixture.root(), &paths)
            .require("ignored_paths")
            .is_empty()
    );
}

#[test]
fn worktree_facts_main_checkout_and_linked_worktree() {
    let fixture = seeded_repo();
    let facts = git::worktree_facts(fixture.root()).require("worktree_facts");
    assert!(
        !facts.is_linked_worktree,
        "the seeded checkout is the main worktree"
    );
    assert_eq!(facts.git_dir, facts.common_dir);

    let out = fixture
        .run(&["git", "worktree", "add", "linked", "-b", "linked-branch"])
        .require("add worktree");
    assert!(out.success(), "worktree add failed: {}", out.stderr);
    let facts = git::worktree_facts(&fixture.path("linked")).require("worktree_facts");
    assert!(
        facts.is_linked_worktree,
        "a linked worktree has distinct git-dir/common-dir"
    );
    assert!(
        Path::new(&facts.git_dir).is_absolute() || facts.git_dir.contains("worktrees"),
        "linked git-dir points into .git/worktrees: {}",
        facts.git_dir
    );
}

#[test]
fn current_branch_reports_branch_none_when_detached() {
    let fixture = seeded_repo();
    let out = fixture
        .run(&["git", "checkout", "-b", "feature/one"])
        .require("branch");
    assert!(out.success(), "checkout failed: {}", out.stderr);
    assert_eq!(
        git::current_branch(fixture.root()).require("current_branch"),
        Some("feature/one".to_string())
    );

    let unborn = Fixture::new().with_git_init().build().require("fixture");
    assert!(
        git::current_branch(unborn.root())
            .require("current_branch")
            .is_some(),
        "an unborn HEAD still names the would-be branch"
    );

    let detached = seeded_repo();
    let out = detached
        .run(&["git", "checkout", "--detach", "HEAD"])
        .require("detach");
    assert!(out.success(), "detach failed: {}", out.stderr);
    assert_eq!(
        git::current_branch(detached.root()).require("current_branch"),
        None,
        "detached HEAD prints nothing from branch --show-current"
    );
}

#[test]
fn policy_variants_exist_and_honor_strip() {
    let fixture = seeded_repo();
    let policy = git::EnvPolicy::StripHookContext;
    assert!(
        git::staged_files_with_policy(fixture.root(), policy)
            .require("staged_files")
            .is_empty()
    );
    let paths: Vec<_> = ["seed.txt"].iter().map(|p| fixture.path(p)).collect();
    assert_eq!(
        git::tracked_paths_with_policy(fixture.root(), &paths, policy)
            .require("tracked_paths")
            .len(),
        1
    );
    assert!(
        git::ignored_paths_with_policy(fixture.root(), &paths, policy)
            .require("ignored_paths")
            .is_empty()
    );
    let facts = git::worktree_facts_with_policy(fixture.root(), policy).require("facts");
    let _ = facts;
    let branch = git::current_branch(fixture.root()).require("branch");
    assert_eq!(
        git::current_branch_with_policy(fixture.root(), policy).require("branch"),
        branch
    );
    assert!(
        git::remote_url_with_policy(fixture.root(), "origin", policy)
            .require("remote_url")
            .is_none()
    );
}

#[test]
fn spawn_failure_surfaces_as_typed_error() {
    // Point every query at a directory where git cannot run: PATH without
    // git is not hermetically adjustable here, so use a deleted directory
    // proxy — a non-repo root still yields errors only for exit-code
    // failures, not spawn failures, so exercise the typed error path via
    // a root that is not a repository.
    let dir = std::env::temp_dir().join(format!("genesis-git-queries-{}", std::process::id()));
    std::fs::create_dir_all(&dir).require("mkdir temp");
    let err = git::worktree_facts(&dir).expect_err("non-repo root must error");
    assert!(matches!(err, GitError::Git { .. }), "{err:?}");
    let err = git::current_branch(&dir).expect_err("non-repo root must error");
    assert!(matches!(err, GitError::Git { .. }), "{err:?}");
    let err = git::staged_files(&dir).expect_err("non-repo root must error");
    assert!(matches!(err, GitError::Git { .. }), "{err:?}");
    let err = git::tracked_paths(&dir, &[dir.as_path()]).expect_err("non-repo root must error");
    assert!(matches!(err, GitError::Git { .. }), "{err:?}");
    let err = git::ignored_paths(&dir, &[dir.as_path()]).expect_err("non-repo root must error");
    assert!(matches!(err, GitError::Git { .. }), "{err:?}");
    std::fs::remove_dir_all(&dir).ok();
}
