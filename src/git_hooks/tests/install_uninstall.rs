//! Install/uninstall ownership-marker tests (design D1/D6).

use super::*;

#[test]
fn install_writes_executable_hook_with_marker_and_script() {
    let fixture = install_fixture();
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    let hook_path = fixture.root().join(".git/hooks/pre-commit");
    let contents = std::fs::read_to_string(&hook_path).require("value");
    assert!(contents.contains(TEST_MARKER), "marker missing: {contents}");
    assert!(contents.contains("exec true"), "script missing: {contents}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&hook_path)
            .require("value")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755, "hook must be executable 0755");
    }
}

#[test]
fn install_creates_hooks_dir_on_demand() {
    let fixture = install_fixture();
    std::fs::remove_dir_all(fixture.root().join(".git/hooks")).require("value");
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    assert!(fixture.root().join(".git/hooks/pre-commit").exists());
}

#[test]
fn install_respects_core_hooks_path() {
    let fixture = install_fixture();
    let out = fixture
        .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
        .require("value");
    assert!(out.success(), "git config failed: {}", out.stderr);
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    assert!(fixture.root().join(".githooks/pre-commit").exists());
    assert!(!fixture.root().join(".git/hooks/pre-commit").exists());
}

#[test]
fn install_refuses_foreign_hook_unmodified() {
    let fixture = install_fixture();
    let hook_path = fixture.root().join(".git/hooks/pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\necho mine\n").require("value");
    let err = install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .unwrap_err();
    assert!(
        matches!(err, GitHooksError::ForeignHook { .. }),
        "got: {err}"
    );
    let msg = err.to_string();
    assert!(msg.contains("refusing"), "error should explain: {msg}");
    assert_eq!(
        std::fs::read_to_string(&hook_path).require("value"),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn install_refuses_empty_hook_file() {
    // Design D6: a 0-byte hook file is foreign — refuse rather than
    // silently overwrite a file the user created.
    let fixture = install_fixture();
    std::fs::write(fixture.root().join(".git/hooks/pre-commit"), "").require("value");
    let err = install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .unwrap_err();
    assert!(
        matches!(err, GitHooksError::ForeignHook { .. }),
        "got: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).require("value"),
        ""
    );
}

#[test]
fn install_is_idempotent_for_own_hooks() {
    let fixture = install_fixture();
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    let v2 = "#!/usr/bin/env sh\nexec false\n";
    install(fixture.root(), HookName::PreCommit, TEST_MARKER, v2).require("value");
    let contents =
        std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).require("value");
    assert!(contents.contains(TEST_MARKER));
    assert!(contents.contains("exec false"));
}

#[test]
fn install_marker_follows_shebang() {
    // Pretender's convention: shebang stays line 1, marker after it.
    let fixture = install_fixture();
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    let contents =
        std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).require("value");
    let mut lines = contents.lines();
    assert_eq!(lines.next(), Some("#!/usr/bin/env sh"));
    assert_eq!(lines.next(), Some(TEST_MARKER));
}

// -- Uninstall -------------------------------------------------------

#[test]
fn uninstall_removes_owned_hook() {
    let fixture = install_fixture();
    install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        TEST_SCRIPT,
    )
    .require("value");
    uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).require("value");
    assert!(!fixture.root().join(".git/hooks/pre-commit").exists());
}

#[test]
fn uninstall_is_noop_for_missing_hook() {
    let fixture = install_fixture();
    let path = uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).require("value");
    assert!(!path.exists(), "no-op uninstall must not create the hook");
}

#[test]
fn uninstall_refuses_foreign_hook() {
    let fixture = install_fixture();
    let hook_path = fixture.root().join(".git/hooks/pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\necho mine\n").require("value");
    let err = uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).unwrap_err();
    assert!(
        matches!(err, GitHooksError::ForeignHook { .. }),
        "got: {err}"
    );
    assert!(hook_path.exists());
}
