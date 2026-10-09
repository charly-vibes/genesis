//! Module-level contract tests for `genesis::git`:
//!
//! - read-only plumbing only: no write-path API (`init`/`add`/`commit`)
//! - subprocess-only substrate: no git library in the dependency graph

use std::path::{Path, PathBuf};

use genesis::git::{self, EnvPolicy, GitError, GitOutput};

/// Source of the `git` module, inspected for write-path commands.
const GIT_SRC: &str = include_str!("../src/git.rs");

#[test]
fn no_write_path_api() {
    // Enumerate the public API surface with compile-checked shapes: the
    // module exposes read-only queries only.
    let _run: fn(&Path, EnvPolicy, &[&str]) -> Result<GitOutput, GitError> = git::run;
    let _repo_root: fn() -> Result<PathBuf, GitError> = git::repo_root;
    let _repo_root_from: fn(&Path) -> Result<PathBuf, GitError> = git::repo_root_from;
    let _changed_files: fn(&Path) -> Result<Vec<String>, GitError> = git::changed_files;
    let _changed_between: fn(&Path, &str, &str) -> Result<Vec<String>, GitError> =
        git::changed_files_between;
    let _uncommitted: fn(&Path) -> Result<Vec<String>, GitError> = git::uncommitted_files;
    let _uncommitted_lossy: fn(&Path) -> Vec<String> = git::uncommitted_files_lossy;
    let _tracked: fn(&Path, &Path) -> Result<bool, GitError> = git::tracked;
    let _is_ignored: fn(&Path, &Path) -> Result<bool, GitError> = git::is_ignored;
    let _content_hash: fn(&Path, &Path) -> Result<String, GitError> = git::content_hash;
    let _committed_hash: fn(&Path, &Path) -> Result<String, GitError> = git::committed_content_hash;
    let _parse_porcelain: fn(&str) -> Vec<String> = git::parse_porcelain;

    // No git invocation in the module may build a write-path command:
    // the string literals for `git init`, `git add`, and `git commit`
    // arguments must never appear in the source.
    for forbidden in ["\"init\"", "\"add\"", "\"commit\""] {
        assert!(
            !GIT_SRC.contains(forbidden),
            "write-path command argument {forbidden} must not appear in src/git.rs"
        );
    }
}

#[test]
fn no_git_library_in_dependency_graph() {
    // Neither genesis's own manifest nor the resolved lockfile may carry
    // git2 or gitoxide — the module is subprocess-only by contract.
    const MANIFEST: &str = include_str!("../Cargo.toml");
    const LOCKFILE: &str = include_str!("../Cargo.lock");
    for source in [MANIFEST, LOCKFILE] {
        for forbidden in ["name = \"git2\"", "name = \"gitoxide\""] {
            assert!(
                !source.contains(forbidden),
                "git library {forbidden} must not appear in the dependency graph"
            );
        }
    }
    assert!(
        !MANIFEST.contains("git2 =") && !MANIFEST.contains("gitoxide ="),
        "git library dependencies must not appear in Cargo.toml"
    );
}
