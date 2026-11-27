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

#[test]
fn install_shebang_only_script_still_execs() {
    // genesis-5sm: a shebang-only script has no trailing newline — the
    // marker must follow the shebang line, never replace it as line 1
    // (the kernel would exec the marker comment as interpreter).
    let fixture = install_fixture();
    let path = install(
        fixture.root(),
        HookName::PreCommit,
        TEST_MARKER,
        "#!/bin/sh",
    )
    .require("value");
    let contents = std::fs::read_to_string(&path).require("value");
    let mut lines = contents.lines();
    assert_eq!(lines.next(), Some("#!/bin/sh"), "shebang stays line 1");
    assert_eq!(lines.next(), Some(TEST_MARKER));
    // And the hook still execs: the interpreter line is intact. `sh`
    // with an empty body reads stdin — pin it closed so the test cannot
    // block.
    let status = std::process::Command::new(&path)
        .stdin(std::process::Stdio::null())
        .status()
        .require("spawn installed hook");
    assert!(status.success(), "shebang-only hook must exec: {status}");
}

#[test]
fn install_refuses_non_utf8_foreign_hook_as_foreign() {
    // genesis-5sm: a hook that is not valid UTF-8 carries no readable
    // marker — it is foreign, not an Io error.
    let fixture = install_fixture();
    let hook_path = fixture.root().join(".git/hooks/pre-commit");
    std::fs::write(&hook_path, [0xFFu8, 0xFE, 0x00]).require("value");
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
        std::fs::read(&hook_path).require("value"),
        vec![0xFFu8, 0xFE, 0x00],
        "file must not be modified"
    );
}

#[test]
fn install_rejects_path_like_hook_names() {
    // genesis-5sm: HookName::Other names carrying ParentDir/RootDir
    // components would escape the hooks directory on join — refuse.
    let fixture = install_fixture();
    for name in ["../crontab", "/etc/passwd", "..", "sub/../pre-commit"] {
        let err = install(
            fixture.root(),
            HookName::Other(name.to_string()),
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::InvalidHookName { .. }),
            "{name}: got {err}"
        );
    }
    assert!(
        !fixture.root().join(".git/hooks/pre-commit").exists(),
        "no hook may be written for a rejected name"
    );
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
