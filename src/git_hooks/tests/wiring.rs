//! Lefthook `ensure_wired` / `is_wired` tests (tasks 4.1-4.5).

use super::*;
use crate::managed_block::BlockDef;

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
