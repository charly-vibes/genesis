//! Contract tests for the remote `genesis::git` queries: `remote_url`
//! and `remote_key`.
//!
//! Spec: `remote_url` answers None (not an error) for an absent remote;
//! `remote_key` is byte-identical to the redactor's former
//! `reduce_git_remote_url` across scp/ssh/https/PAT remote URL forms.

mod common;

use common::TestUnwrap;

use genesis::fixture::Fixture;
use genesis::git;

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
fn remote_url_reports_remote_and_none_when_absent() {
    let fixture = seeded_repo();
    let out = fixture
        .run(&[
            "git",
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .require("add remote");
    assert!(out.success(), "remote add failed: {}", out.stderr);
    assert_eq!(
        git::remote_url(fixture.root(), "origin").require("remote_url"),
        Some("https://github.com/owner/repo.git".to_string())
    );
    assert_eq!(
        git::remote_url(fixture.root(), "missing").require("remote_url"),
        None,
        "a non-existent remote is None, not an error"
    );
}

#[test]
fn remote_key_matches_whisper_canonical_key_forms() {
    // scp, ssh, https, and PAT forms — the fixture set the whisper
    // canonical_key equivalence is specified on.
    let cases = [
        ("git@github.com:owner/repo.git", "github.com/owner/repo"),
        (
            "ssh://git@github.com/owner/repo.git",
            "github.com/owner/repo",
        ),
        ("https://github.com/owner/repo.git", "github.com/owner/repo"),
        (
            "https://pat@github.com/owner/repo.git",
            "github.com/owner/repo",
        ),
        ("https://github.com/owner/repo", "github.com/owner/repo"),
        (
            "https://github.com/owner/repo?foo=bar",
            "github.com/owner/repo",
        ),
        (
            "http://user:pass@host.tld/org/repo.git",
            "host.tld/org/repo",
        ),
    ];
    for (url, expected) in cases {
        assert_eq!(git::remote_key(url), expected, "remote_key({url})");
        // The redactor re-export stays byte-identical.
        assert_eq!(
            genesis::feedback::redactor::reduce_git_remote_url(url),
            expected,
            "redactor re-export({url})"
        );
    }
}
