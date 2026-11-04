//! Unit tests for `git_hooks` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use crate::fixture::Fixture;

// -- Scope isolation --------------------------------------------------

/// Env pairs that point git's global/system config scopes at absent
/// fixture-local paths. Without this, tests inherit the machine's real
/// global `core.hooksPath` (the exact scenario genesis-c64 fixes) and
/// the default-path expectations break. Rust runs tests in parallel
/// threads, so mutating process env is not an option.
fn isolated_env(root: &Path) -> Vec<(String, String)> {
    let global = root.join(".git/genesis-test-absent-global-gitconfig");
    let system = root.join(".git/genesis-test-absent-system-gitconfig");
    vec![
        (
            "GIT_CONFIG_GLOBAL".to_string(),
            global.to_string_lossy().into_owned(),
        ),
        (
            "GIT_CONFIG_SYSTEM".to_string(),
            system.to_string_lossy().into_owned(),
        ),
    ]
}

// Local wrappers: same names as the public API, but with git's
// global/system scopes isolated per fixture (glob imports from
// `super::*` are shadowed by these definitions).
fn resolve_hooks_dir(root: &Path) -> Result<PathBuf, GitHooksError> {
    resolve_hooks_dir_with_env(root, &isolated_env(root))
}

fn install(
    root: &Path,
    hook_name: HookName,
    marker: &str,
    script: &str,
) -> Result<PathBuf, GitHooksError> {
    install_with_env(root, hook_name, marker, script, &isolated_env(root))
}

fn uninstall(root: &Path, hook_name: HookName, marker: &str) -> Result<PathBuf, GitHooksError> {
    uninstall_with_env(root, hook_name, marker, &isolated_env(root))
}

fn owner(root: &Path, hook_name: &HookName) -> Option<Owner> {
    owner_with_env(root, hook_name, &isolated_env(root))
}

fn framework(root: &Path) -> Framework {
    framework_with_env(root, &isolated_env(root))
}

// -- Repository root discovery ---------------------------------------

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

// -- Hooks directory resolution --------------------------------------

#[test]
fn resolve_hooks_dir_defaults_to_git_hooks() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let resolved = resolve_hooks_dir(fixture.root()).require("value");
    assert_eq!(resolved, fixture.root().join(".git/hooks"));
}

#[test]
fn resolve_hooks_dir_honors_local_core_hooks_path() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let out = fixture
        .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
        .require("value");
    assert!(out.success(), "git config failed: {}", out.stderr);
    let resolved = resolve_hooks_dir(fixture.root()).require("value");
    assert_eq!(resolved, fixture.root().join(".githooks"));
}

// -- Multi-scope resolution (genesis-c64) ----------------------------

/// Writes a global git config setting `core.hooksPath` and returns the
/// env pairs that point git's global/system scopes at fixture-local
/// files (avoids touching process env, which is racy across tests).
fn global_scope_env(fixture: &Fixture, hooks_path: &str) -> Vec<(String, String)> {
    let global_cfg = fixture.path("global-gitconfig");
    std::fs::write(&global_cfg, format!("[core]\n\thooksPath = {hooks_path}\n")).require("value");
    let system_cfg = fixture.path("system-gitconfig");
    std::fs::write(&system_cfg, "").require("value");
    vec![
        (
            "GIT_CONFIG_GLOBAL".to_string(),
            global_cfg.to_string_lossy().into_owned(),
        ),
        (
            "GIT_CONFIG_SYSTEM".to_string(),
            system_cfg.to_string_lossy().into_owned(),
        ),
    ]
}

fn effective_with_env(
    root: &Path,
    envs: &[(String, String)],
) -> Result<EffectiveHooksDir, GitHooksError> {
    effective_hooks_dir_with_env(root, envs)
}

#[test]
fn resolve_hooks_dir_honors_global_core_hooks_path() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let envs = global_scope_env(&fixture, ".global-hooks");
    let resolved = effective_with_env(fixture.root(), &envs).require("value");
    assert_eq!(resolved.path, fixture.root().join(".global-hooks"));
    assert_eq!(resolved.scope, HooksDirScope::Global);
    // resolve_hooks_dir agrees when it sees the same env.
    assert_eq!(
        resolve_hooks_dir_with_env(fixture.root(), &envs).require("value"),
        fixture.root().join(".global-hooks"),
        "multi-scope resolution must not drop the global value"
    );
}

#[test]
fn local_overrides_global_core_hooks_path() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let envs = global_scope_env(&fixture, ".global-hooks");
    let out = fixture
        .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
        .require("value");
    assert!(out.success(), "git config failed: {}", out.stderr);
    let resolved = effective_with_env(fixture.root(), &envs).require("value");
    assert_eq!(resolved.path, fixture.root().join(".githooks"));
    assert_eq!(resolved.scope, HooksDirScope::Local);
}

#[test]
fn resolve_hooks_dir_honors_system_core_hooks_path() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    // Empty GIT_CONFIG_GLOBAL + populated GIT_CONFIG_SYSTEM isolates the
    // system scope.
    let system_cfg = fixture.path("system-gitconfig");
    std::fs::write(&system_cfg, "[core]\n\thooksPath = .system-hooks\n").require("value");
    let envs = vec![
        (
            "GIT_CONFIG_GLOBAL".to_string(),
            fixture
                .path("absent-gitconfig")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "GIT_CONFIG_SYSTEM".to_string(),
            system_cfg.to_string_lossy().into_owned(),
        ),
    ];
    let resolved = effective_with_env(fixture.root(), &envs).require("value");
    assert_eq!(resolved.path, fixture.root().join(".system-hooks"));
    assert_eq!(resolved.scope, HooksDirScope::System);
}

#[test]
fn effective_hooks_dir_reports_default_scope_when_unset() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let envs = vec![
        (
            "GIT_CONFIG_GLOBAL".to_string(),
            fixture.path("absent-global").to_string_lossy().into_owned(),
        ),
        (
            "GIT_CONFIG_SYSTEM".to_string(),
            fixture.path("absent-system").to_string_lossy().into_owned(),
        ),
    ];
    let resolved = effective_with_env(fixture.root(), &envs).require("value");
    assert_eq!(resolved.scope, HooksDirScope::Default);
    assert_eq!(resolved.path, fixture.root().join(".git/hooks"));
}

#[test]
fn effective_hooks_dir_reports_local_scope() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let out = fixture
        .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
        .require("value");
    assert!(out.success(), "git config failed: {}", out.stderr);
    let resolved = effective_with_env(fixture.root(), &[]).require("value");
    assert_eq!(resolved.scope, HooksDirScope::Local);
    assert_eq!(resolved.path, fixture.root().join(".githooks"));
}

#[test]
fn effective_hooks_dir_surfaces_disabled_for_empty_string() {
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let out = fixture
        .run(&["git", "config", "--local", "core.hooksPath", ""])
        .require("value");
    assert!(out.success(), "git config failed: {}", out.stderr);
    let resolved = effective_with_env(fixture.root(), &[]).require("value");
    assert_eq!(resolved.scope, HooksDirScope::Disabled);
    // resolve_hooks_dir keeps its documented fallback for callers.
    assert_eq!(
        resolve_hooks_dir(fixture.root()).require("value"),
        fixture.root().join(".git/hooks")
    );
}

// -- Hook file names -------------------------------------------------

#[test]
fn hook_name_file_name_mapping() {
    assert_eq!(HookName::PreCommit.file_name(), "pre-commit");
    assert_eq!(HookName::PrePush.file_name(), "pre-push");
    assert_eq!(
        HookName::Other("commit-msg".to_string()).file_name(),
        "commit-msg"
    );
}

// -- Install ---------------------------------------------------------

fn install_fixture() -> Fixture {
    match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    }
}

const TEST_MARKER: &str = "# Installed by test-tool.";
const TEST_SCRIPT: &str = "#!/usr/bin/env sh\nexec true\n";

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
    uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).require("value");
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

// -- Owner detection --------------------------------------------------

fn write_hook(fixture: &Fixture, hook: HookName, content: &str) {
    let path = fixture.root().join(".git/hooks").join(hook.file_name());
    std::fs::create_dir_all(path.parent().require("value")).require("value");
    std::fs::write(path, content).require("value");
}

#[test]
fn owner_detects_each_sigil_in_table_order() {
    let cases: &[(&str, Owner)] = &[
        ("#!/bin/sh\n# managed by lefthook\n", Owner::Lefthook),
        (
            "#!/bin/sh\n. \"$(dirname -- \"$0\")\"/_/husky.sh\n",
            Owner::Husky,
        ),
        (
            "#!/bin/sh\nexec bd hooks run pre-commit \"$@\"\n",
            Owner::Bd,
        ),
        (
            "#!/bin/sh\n# installed by pre-commit framework\n", // generic
            Owner::PreCommit,
        ),
        (
            "#!/bin/sh\n# managed by prek\nexec prek run pre-commit\n",
            Owner::Prek,
        ),
    ];
    for (content, expected) in cases {
        let fixture = install_fixture();
        write_hook(&fixture, HookName::PreCommit, content);
        let got = owner(fixture.root(), &HookName::PreCommit)
            .unwrap_or_else(|| panic!("expected {expected:?} for {content:?}"));
        assert_eq!(got, *expected, "content: {content:?}");
    }
}

#[test]
fn owner_most_specific_sigil_wins_bd_shim_chaining_prek() {
    // wai test: a bd hook that chains prek must report bd, not prek.
    let fixture = install_fixture();
    write_hook(
        &fixture,
        HookName::PreCommit,
        "#!/usr/bin/env sh\n# bd-shim v2\n# chains prek\nexec prek run pre-commit\n",
    );
    assert_eq!(owner(fixture.root(), &HookName::PreCommit), Some(Owner::Bd));
}

#[test]
fn owner_none_for_missing_or_unknown_hook() {
    let fixture = install_fixture();
    assert_eq!(owner(fixture.root(), &HookName::PreCommit), None);
    write_hook(&fixture, HookName::PreCommit, "#!/bin/sh\necho custom\n");
    assert_eq!(owner(fixture.root(), &HookName::PreCommit), None);
}

#[test]
fn owner_resolves_hooks_dir() {
    // Detection must honor core.hooksPath like the rest of the module.
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    fixture
        .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
        .require("value");
    let delegated = fixture.root().join(".githooks");
    std::fs::create_dir_all(&delegated).require("value");
    std::fs::write(delegated.join("pre-commit"), "# managed by lefthook\n").require("value");
    assert_eq!(
        owner(fixture.root(), &HookName::PreCommit),
        Some(Owner::Lefthook)
    );
}

// -- Framework detection ----------------------------------------------

#[test]
fn framework_lefthook_config_present() {
    for name in ["lefthook.yml", "lefthook.yaml"] {
        let fixture = install_fixture();
        std::fs::write(fixture.root().join(name), "pre-commit:\n  commands:\n").require("value");
        assert_eq!(framework(fixture.root()), Framework::Lefthook, "{name}");
    }
}

#[test]
fn framework_prek_config_present() {
    let fixture = install_fixture();
    std::fs::write(fixture.root().join("prek.toml"), "").require("value");
    assert_eq!(framework(fixture.root()), Framework::Prek);
}

#[test]
fn framework_husky_via_hook_sigil_without_root_config() {
    let fixture = install_fixture();
    write_hook(
        &fixture,
        HookName::PreCommit,
        "#!/bin/sh\n. \"$(dirname -- \"$0\")\"/_/husky.sh\n",
    );
    assert_eq!(framework(fixture.root()), Framework::Husky);
}

#[test]
fn framework_none_without_signals() {
    let fixture = install_fixture();
    assert_eq!(framework(fixture.root()), Framework::None);
}

#[test]
fn framework_precedence_lefthook_then_prek_then_husky() {
    // Lefthook beats everything.
    let fixture = install_fixture();
    std::fs::write(fixture.root().join("lefthook.yml"), "").require("value");
    std::fs::write(fixture.root().join("prek.toml"), "").require("value");
    write_hook(&fixture, HookName::PreCommit, "# husky\n");
    assert_eq!(framework(fixture.root()), Framework::Lefthook);

    // Prek beats husky sigils.
    let fixture = install_fixture();
    std::fs::write(fixture.root().join("prek.toml"), "").require("value");
    write_hook(&fixture, HookName::PreCommit, "# husky\n");
    assert_eq!(framework(fixture.root()), Framework::Prek);
}

// -- Lefthook wiring (tasks 4.1-4.5) ----------------------------------

mod wiring_tests {
    use super::*;
    use crate::managed_block::BlockDef;

    fn ah_block() -> BlockDef {
        BlockDef::new("AH")
    }

    fn ah_content() -> &'static str {
        // Caller content is the ENTRIES INSIDE the stage's `commands:`
        // mapping (genesis-r99): inserted verbatim after the `commands:`
        // line, so it carries the commands:-child indent (4 spaces).
        // Leading \n keeps the start marker on its own line.
        "\n    ah:\n      run: ah check\n"
    }

    fn basic_config() -> &'static str {
        "pre-commit:\n  commands:\n    lint:\n      run: lint\npre-push:\n  commands:\n    test:\n      run: test\n"
    }

    fn config_fixture(contents: &str) -> Fixture {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        std::fs::write(fixture.root().join("lefthook.yml"), contents).require("value");
        fixture
    }

    #[test]
    fn ensure_wired_nests_block_inside_existing_commands_mapping() {
        // genesis-r99: lefthook silently ignores stage-level unknown
        // keys — the block MUST land inside the stage's `commands:`
        // mapping, and the stage must not gain a duplicate key.
        let fixture = config_fixture(basic_config());
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let block_text = format!(
            "{}{}{}",
            ah_block().start_marker,
            ah_content(),
            ah_block().end_marker
        );
        let expected = format!(
            "pre-commit:\n  commands:\n{block_text}\n{}",
            &basic_config()["pre-commit:\n  commands:\n".len()..]
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_wired_emits_commands_mapping_when_stage_lacks_one() {
        // Stage exists but has no `commands:` key: create it, then the
        // block, so the wired command is a real lefthook command.
        let config = "pre-commit:\n  skip: true\n";
        let fixture = config_fixture(config);
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let block_text = format!(
            "{}{}{}",
            ah_block().start_marker,
            ah_content(),
            ah_block().end_marker
        );
        // `commands:` is emitted right after the anchor; existing stage
        // children (e.g. `skip:`) follow the block, still stage-level.
        let expected = format!("pre-commit:\n  commands:\n{block_text}\n  skip: true\n");
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_wired_creates_missing_stage_section() {
        // Config has only a pre-commit section; wiring pre-push must
        // append the missing stage section containing the block.
        let config = "pre-commit:\n  commands:\n    lint:\n      run: lint\n";
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        std::fs::write(fixture.root().join("lefthook.yml"), config).require("value");
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PrePush,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let block_text = format!(
            "{}{}{}",
            ah_block().start_marker,
            ah_content(),
            ah_block().end_marker
        );
        assert!(after.starts_with(config));
        assert!(after.contains(&format!("\npre-push:\n  commands:\n{block_text}\n")));
    }

    #[test]
    fn ensure_wired_is_idempotent_when_content_present() {
        let fixture = config_fixture(basic_config());
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let once = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let twice = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(once, twice);
        assert_eq!(twice.matches("AH:START").count(), 1);
    }

    #[test]
    fn ensure_wired_errors_on_missing_config_and_never_creates_one() {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let err = lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no supported hook framework config"),
            "error should explain: {msg}"
        );
        assert!(!fixture.root().join("lefthook.yml").exists());
        assert!(!fixture.root().join("lefthook.yaml").exists());
    }

    #[test]
    fn ensure_wired_prefers_yml_when_both_exist() {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        std::fs::write(fixture.root().join("lefthook.yml"), basic_config()).require("value");
        std::fs::write(fixture.root().join("lefthook.yaml"), "other: file\n").require("value");
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let yml = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let yaml = std::fs::read_to_string(fixture.root().join("lefthook.yaml")).require("value");
        assert!(yml.contains("AH:START"), ".yml should be modified");
        assert_eq!(yaml, "other: file\n", ".yaml must be ignored (D6)");
    }

    #[test]
    fn ensure_wired_errors_unmodified_on_quoted_stage_key() {
        let quoted = "\"pre-push\":\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(quoted);
        let err = lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PrePush,
            &ah_block(),
            ah_content(),
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
            "got: {err}"
        );
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(after, quoted, "config must not be modified");
    }

    #[test]
    fn ensure_wired_anchors_past_comment_lines_mentioning_the_stage() {
        // GH#28 item 1: comments mentioning `pre-commit` before the real
        // column-0 key must not trip the contains-refusal (D6) — the
        // stage is anchorable.
        let config = "# pre-commit hooks are managed by ah\npre-commit:\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(config);
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert!(after.contains("AH:START"), "block should be wired");
        assert!(
            after.starts_with(
                "# pre-commit hooks are managed by ah\npre-commit:\n  commands:\n<!--"
            ),
            "block must nest inside commands:, comment preserved\n---\n{after}"
        );
    }

    #[test]
    fn ensure_wired_treats_comment_only_stage_mention_as_absent() {
        // Stage name appears only in a comment: not present (D6 refusal
        // must not fire), so the section is appended.
        let config = "# no pre-commit stage yet, see docs\nother: key\n";
        let fixture = config_fixture(config);
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert!(after.contains("AH:START"), "block should be wired");
    }

    #[test]
    fn injected_block_is_recognized_by_managed_block_reader() {
        let fixture = config_fixture(basic_config());
        lefthook::ensure_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            &ah_block(),
            ah_content(),
        )
        .require("value");
        let mut reg = crate::managed_block::BlockRegistry::new();
        reg.register(ah_block());
        let injector = crate::managed_block::BlockInjector::new(reg);
        let path = fixture.root().join("lefthook.yml");
        assert!(injector.has_block(&path, "AH"));
        let read = injector.read_block(&path, "AH").require("value");
        assert!(read.contains(ah_block().start_marker.as_str()));
        assert!(read.contains("run: ah check"));
    }

    #[test]
    fn is_wired_true_when_command_in_target_stage() {
        let fixture = config_fixture(
            "pre-commit:\n  commands:\n    ah:\n      run: ah check\npre-push:\n  commands:\n    test:\n      run: test\n",
        );
        assert!(lefthook::is_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            "ah check"
        ));
        assert!(!lefthook::is_wired(
            fixture.root(),
            lefthook::Stage::PrePush,
            "ah check"
        ));
    }

    #[test]
    fn is_wired_false_when_stage_missing_or_no_config() {
        let fixture = config_fixture(basic_config());
        assert!(!lefthook::is_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            "ah check"
        ));
        let empty = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        assert!(!lefthook::is_wired(
            empty.root(),
            lefthook::Stage::PreCommit,
            "ah check"
        ));
    }

    #[test]
    fn is_wired_command_in_wrong_stage_is_not_wired() {
        let fixture = config_fixture("pre-push:\n  commands:\n    ah:\n      run: ah check\n");
        assert!(lefthook::is_wired(
            fixture.root(),
            lefthook::Stage::PrePush,
            "ah check"
        ));
        assert!(!lefthook::is_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            "ah check"
        ));
    }

    // === ensure_command_wired (genesis-au8) ===

    /// Command-level wiring fixture: comment-prefixed markers (the
    /// YAML convention) around a generic command name/run.
    fn cmd_block() -> BlockDef {
        BlockDef::with_markers("T", "# <!-- T:START -->", "# <!-- T:END -->")
    }

    const CMD_NAME: &str = "tcmd";
    const CMD_RUN: &str = "echo hi";

    /// The marker-guarded entry lines at `indent` (mapping-entry
    /// level); every line ends with a newline.
    fn cmd_entry(indent: usize) -> String {
        let pad = " ".repeat(indent);
        format!(
            "{pad}# <!-- T:START -->\n{pad}{CMD_NAME}:\n{pad}  run: {CMD_RUN}\n{pad}# <!-- T:END -->\n"
        )
    }

    /// The full `commands:` wrapper at `indent` (stage-children
    /// level), entry nested at indent + 2.
    fn cmd_wrapper(indent: usize) -> String {
        format!("{}commands:\n{}", " ".repeat(indent), cmd_entry(indent + 2))
    }

    #[test]
    fn ensure_command_wired_inserts_inside_existing_commands_mapping() {
        let fixture = config_fixture(basic_config());
        let outcome = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        assert_eq!(outcome, lefthook::WiredOutcome::Injected);
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let expected = format!(
            "pre-commit:\n  commands:\n{}{}",
            cmd_entry(4),
            &basic_config()["pre-commit:\n  commands:\n".len()..]
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_command_wired_infers_entry_indent_from_mapping_children() {
        // Per-level YAML indent is config-dependent: a 4-space config
        // nests entries at commands+4, not +2.
        let config = "pre-commit:\n    commands:\n        lint:\n            run: lint\n";
        let fixture = config_fixture(config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let expected = format!(
            "pre-commit:\n    commands:\n{}        lint:\n            run: lint\n",
            cmd_entry(8)
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_command_wired_empty_mapping_defaults_to_commands_indent_plus_two() {
        let config =
            "pre-commit:\n  commands:\npre-push:\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let expected = format!(
            "pre-commit:\n  commands:\n{}pre-push:\n  commands:\n    test:\n      run: test\n",
            cmd_entry(4)
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_command_wired_creates_wrapper_when_stage_is_empty() {
        let config = "pre-commit:\npre-push:\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let expected = format!(
            "pre-commit:\n{}pre-push:\n  commands:\n    test:\n      run: test\n",
            cmd_wrapper(2)
        );
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_command_wired_appends_missing_stage_section() {
        // Config has only a pre-push section; wiring pre-commit must
        // append the missing stage section with the wrapper.
        let config = "pre-push:\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert!(after.starts_with(config));
        assert_eq!(
            after[config.len()..],
            format!("pre-commit:\n{}", cmd_wrapper(2))
        );
    }

    #[test]
    fn ensure_command_wired_wraps_commands_into_stage_with_children() {
        // Stage exists with children but no `commands:` key — the
        // wrapper goes directly after the anchor at the children indent.
        let config = "pre-commit:\n  parallel: true\n";
        let fixture = config_fixture(config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let expected = format!("pre-commit:\n{}  parallel: true\n", cmd_wrapper(2));
        assert_eq!(after, expected);
    }

    #[test]
    fn ensure_command_wired_is_idempotent_when_markers_present() {
        let fixture = config_fixture(basic_config());
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let once = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        let outcome = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        assert_eq!(outcome, lefthook::WiredOutcome::AlreadyWired);
        let twice = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(once, twice);
        assert_eq!(twice.matches("T:START").count(), 1);
    }

    #[test]
    fn ensure_command_wired_refuses_mismatched_commands_indent() {
        // children at indent 6 but `commands:` at indent 2 —
        // unrecognized structure, refuse unmodified (honest_anchor).
        let config = "pre-commit:\n      parallel: true\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(config);
        let err = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
            "got: {err}"
        );
        assert!(err.to_string().contains("commands key is at indent"));
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(after, config, "config must not be modified");
    }

    #[test]
    fn ensure_command_wired_errors_unmodified_on_quoted_stage_key() {
        let quoted = "\"pre-commit\":\n  commands:\n    test:\n      run: test\n";
        let fixture = config_fixture(quoted);
        let err = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
            "got: {err}"
        );
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(after, quoted, "config must not be modified");
    }

    #[test]
    fn ensure_command_wired_errors_on_missing_config_and_never_creates_one() {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let err = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::MissingLefthookConfig { .. }),
            "got: {err}"
        );
        assert!(!fixture.root().join("lefthook.yml").exists());
    }

    #[test]
    fn ensure_command_wired_refuses_unbalanced_markers_unmodified() {
        // Exactly one marker present — unknown state, refuse rather
        // than guess (same idempotence contract as spk hooks install).
        let config = "pre-commit:\n  commands:\n    lint:\n      run: lint\n# <!-- T:START -->\n";
        let fixture = config_fixture(config);
        let err = lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::UnbalancedLefthookMarkers { .. }),
            "got: {err}"
        );
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(after, config, "config must not be modified");
    }

    #[test]
    fn ensure_command_wired_keeps_markers_on_their_own_lines() {
        // Guard against the ensure_wired glue class: the END marker
        // must be followed by a newline, never glued onto the next
        // existing line — with comment-prefixed markers that would
        // turn the next line into a YAML comment (silent key loss).
        let fixture = config_fixture(basic_config());
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert!(
            after.contains("# <!-- T:END -->\n    lint:\n"),
            "END marker must be followed by a newline, not the next key: {after}"
        );
        for line in after.lines() {
            if line.contains("T:START") || line.contains("T:END") {
                assert_eq!(
                    line.trim(),
                    if line.contains("T:START") {
                        "# <!-- T:START -->"
                    } else {
                        "# <!-- T:END -->"
                    },
                    "marker must sit alone on its line: {line:?}"
                );
            }
        }
    }

    /// Guard (spec: "Tool-specific gates stay in tools"): the
    /// non-test portion of this module must not hardcode consuming-tool
    /// gate commands. Test fixtures may use arbitrary gate strings, so
    /// only the source up to this test module is scanned.
    #[test]
    fn module_source_has_no_tool_gate_strings() {
        const SOURCE: &str = include_str!("../git_hooks.rs");
        let production = SOURCE
            .split_once("#[cfg(test)]")
            .expect("test module marker must exist")
            .0;
        for gate in [
            "ah check",
            "pretender check",
            "testaruda select",
            "just check-claims",
        ] {
            assert!(
                !production.contains(gate),
                "git_hooks module must not hardcode the consuming-tool gate {gate:?}"
            );
        }
    }
}
