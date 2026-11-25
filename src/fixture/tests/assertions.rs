//! Tests for `Fixture`'s assertion helpers and the git-init integration.

// pretender: role=test
use crate::fixture::{Fixture, FixtureError};
use crate::testutil::TestUnwrap;

#[test]
fn test_assert_no_file_passes_when_missing() {
    let fixture = Fixture::new().build().require("build");
    assert!(
        !fixture.root().join("nonexistent.txt").exists(),
        "precondition: file must be absent"
    );
    fixture.assert_no_file("nonexistent.txt");
}

#[test]
#[should_panic(expected = "expected file to exist")]
fn test_assert_file_exists_panics_when_missing() {
    let fixture = Fixture::new().build().require("build");
    assert!(
        !fixture.root().join("missing.txt").exists(),
        "precondition: file must be absent"
    );
    fixture.assert_file_exists("missing.txt");
}

#[test]
#[should_panic(expected = "expected file to NOT exist")]
fn test_assert_no_file_panics_when_exists() {
    let fixture = Fixture::new()
        .with_file("present.txt", "")
        .build()
        .require("build");
    assert!(
        fixture.root().join("present.txt").exists(),
        "precondition: file must exist"
    );
    fixture.assert_no_file("present.txt");
}

#[test]
#[should_panic(expected = "expected file 'data.txt' to contain")]
fn test_assert_file_contains_panics_on_mismatch() {
    let fixture = Fixture::new()
        .with_file("data.txt", "hello")
        .build()
        .require("build");
    assert_eq!(
        std::fs::read_to_string(fixture.root().join("data.txt")).require("read data.txt"),
        "hello",
        "precondition: file must hold the mismatching content"
    );
    fixture.assert_file_contains("data.txt", "world");
}

/// Skips when git is unavailable; `Fixture::build` reports that as a `Git` error.
#[test]
fn test_with_git_init_creates_repo() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(FixtureError::Git { .. }) => return,
        Err(e) => panic!("unexpected build error: {e:?}"),
    };
    fixture.assert_marker(".git");
    let status = fixture.run(&["git", "status"]).require("git status");
    assert!(status.success(), "git status failed: {}", status.stderr);
}
