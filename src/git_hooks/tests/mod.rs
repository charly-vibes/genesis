//! Unit tests for `git_hooks`, split per-topic under the pretender
//! file-lines budget.

// pretender: role=test
use crate::testutil::TestUnwrap;

use std::path::{Path, PathBuf};

use super::*;
use crate::fixture::Fixture;
use crate::managed_block::BlockDef;

mod hooks_dir;
mod install_uninstall;
mod owner_framework;
mod root;
mod wiring;
mod wiring_command;

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

pub(super) fn install_fixture() -> Fixture {
    match Fixture::new().with_git_init().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    }
}

pub(super) const TEST_MARKER: &str = "# Installed by test-tool.";
pub(super) const TEST_SCRIPT: &str = "#!/usr/bin/env sh\nexec true\n";

pub(super) fn ah_block() -> BlockDef {
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
