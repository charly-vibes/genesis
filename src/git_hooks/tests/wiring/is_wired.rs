//! `is_wired` tests (design D5: stage-scoped scan semantics).

use super::*;

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
    let empty = built_fixture();
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
