//! Owner and framework detection tests (design D3/D7).

use super::*;

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
