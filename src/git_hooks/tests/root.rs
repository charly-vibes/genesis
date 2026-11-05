//! Repository-root discovery tests.

use super::*;

#[test]
fn repo_root_finds_root_from_nested_dir() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let nested = fixture.path("a/b/c");
    std::fs::create_dir_all(&nested).require("value");
    let found = repo_root_from(&nested).require("value");
    assert_eq!(found, fixture.root());
}

#[test]
fn repo_root_from_the_root_itself_works() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let found = repo_root_from(fixture.root()).require("value");
    assert_eq!(found, fixture.root());
}

#[test]
fn repo_root_errors_outside_a_repository() {
    let fixture = match Fixture::new().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let err = repo_root_from(fixture.root()).unwrap_err();
    assert!(matches!(err, GitHooksError::NotInRepo { .. }));
    let msg = err.to_string();
    assert!(
        msg.contains("not inside a git repository"),
        "error should name the problem: {msg}"
    );
}
