//! Hooks-directory resolution tests (genesis-c64).

use super::*;

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

#[test]
fn query_errors_not_a_repository_outside_a_repo() {
    // genesis-5sm: `git config --local` exits 128 outside a repository —
    // a typed error, never "unset" walking on to global/system.
    let fixture = match Fixture::new().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let err = resolve_hooks_dir(fixture.root()).unwrap_err();
    assert!(
        matches!(err, GitHooksError::NotARepository { .. }),
        "got: {err}"
    );
    assert!(err.to_string().contains("not a git repository"));
}

#[test]
fn query_errors_typed_on_corrupt_config() {
    // genesis-5sm: a corrupt `.git/config` also exits 128 — refuse with
    // a typed error instead of reporting HooksDirScope::Default.
    let fixture = match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    };
    let config = fixture.root().join(".git/config");
    let mut contents = std::fs::read_to_string(&config).require("value");
    contents.push_str("not [valid config\n");
    std::fs::write(&config, contents).require("value");
    let err =
        effective_hooks_dir_with_env(fixture.root(), &isolated_env(fixture.root())).unwrap_err();
    assert!(
        matches!(err, GitHooksError::ConfigParseFailed { .. }),
        "got: {err}"
    );
    assert!(err.to_string().contains("git config failed"));
}

#[test]
fn hook_name_file_name_mapping() {
    assert_eq!(HookName::PreCommit.file_name(), "pre-commit");
    assert_eq!(HookName::PrePush.file_name(), "pre-push");
    assert_eq!(
        HookName::Other("commit-msg".to_string()).file_name(),
        "commit-msg"
    );
}
