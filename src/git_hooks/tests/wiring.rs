//! Lefthook `ensure_wired` / `is_wired` tests (tasks 4.1-4.5).

use super::*;

/// Fixture with no `lefthook.yml` (framework detection must fail).
fn built_fixture() -> Fixture {
    match Fixture::new().build() {
        Ok(f) => f,
        Err(e) => panic!("fixture build failed: {e}"),
    }
}

/// Wire `stage` on `root`, failing the test on error.
fn wire(root: &Path, stage: lefthook::Stage) {
    lefthook::ensure_wired(root, stage, &ah_block(), ah_content()).require("value");
}

/// Read back the wired `.yml` config.
fn wired_config(root: &Path) -> String {
    std::fs::read_to_string(root.join("lefthook.yml")).require("value")
}

/// The managed block verbatim, as it must appear in the config.
fn block_text() -> String {
    format!(
        "{}{}{}",
        ah_block().start_marker,
        ah_content(),
        ah_block().end_marker
    )
}

#[test]
fn ensure_wired_nests_block_inside_existing_commands_mapping() {
    // genesis-r99: lefthook silently ignores stage-level unknown
    // keys — the block MUST land inside the stage's `commands:`
    // mapping, and the stage must not gain a duplicate key.
    let fixture = config_fixture(basic_config());
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let after = wired_config(fixture.root());
    let expected = format!(
        "pre-commit:\n  commands:\n{}\n{}",
        block_text(),
        &basic_config()["pre-commit:\n  commands:\n".len()..]
    );
    assert_eq!(after, expected);
}

#[test]
fn ensure_wired_emits_commands_mapping_when_stage_lacks_one() {
    // Stage exists but has no `commands:` key: create it, then the
    // block, so the wired command is a real lefthook command.
    let fixture = config_fixture("pre-commit:\n  skip: true\n");
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let after = wired_config(fixture.root());
    // `commands:` is emitted right after the anchor; existing stage
    // children (e.g. `skip:`) follow the block, still stage-level.
    let expected = format!("pre-commit:\n  commands:\n{}\n  skip: true\n", block_text());
    assert_eq!(after, expected);
}

#[test]
fn ensure_wired_creates_missing_stage_section() {
    // Config has only a pre-commit section; wiring pre-push must
    // append the missing stage section containing the block.
    let config = "pre-commit:\n  commands:\n    lint:\n      run: lint\n";
    let fixture = config_fixture(config);
    wire(fixture.root(), lefthook::Stage::PrePush);
    let after = wired_config(fixture.root());
    assert!(after.starts_with(config));
    assert!(after.contains(&format!("\npre-push:\n  commands:\n{}\n", block_text())));
}

#[test]
fn ensure_wired_wires_both_stages_of_a_dual_section_config() {
    // Idempotence is per stage section: a file-level check satisfied the
    // second stage from the first stage's block, leaving the common
    // both-sections config with the gate in pre-commit only. Comments are
    // not column-0 keys, so a stage section spans an injected block and
    // the per-section check is exact.
    let fixture = config_fixture(basic_config());
    wire(fixture.root(), lefthook::Stage::PreCommit);
    wire(fixture.root(), lefthook::Stage::PrePush);

    let after = wired_config(fixture.root());
    let push_section = &after[after.find("pre-push:").expect("pre-push section stays")..];
    assert!(
        push_section.contains("run: ah check"),
        "pre-push must run the gate; got:\n{after}"
    );
    assert_eq!(
        after.matches("AH:START").count(),
        2,
        "each stage carries its own managed block; got:\n{after}"
    );
}

#[test]
fn ensure_wired_is_idempotent_when_content_present() {
    let fixture = config_fixture(basic_config());
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let once = wired_config(fixture.root());
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let twice = wired_config(fixture.root());
    assert_eq!(once, twice);
    assert_eq!(twice.matches("AH:START").count(), 1);
}

#[test]
fn ensure_wired_errors_on_missing_config_and_never_creates_one() {
    let fixture = built_fixture();
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
    let fixture = config_fixture(basic_config());
    std::fs::write(fixture.root().join("lefthook.yaml"), "other: file\n").require("value");
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let yml = wired_config(fixture.root());
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
    assert_eq!(
        wired_config(fixture.root()),
        quoted,
        "config must not be modified"
    );
}

#[test]
fn ensure_wired_refuses_unbalanced_markers_unmodified() {
    // genesis-5sm: per-stage blocks legitimately appear once per stage
    // section, so balance is count equality file-wide, not presence —
    // differing start/end counts are an unknown state, refuse rather
    // than guess (same discipline as ensure_command_wired's
    // marker_outcome).
    let config = format!(
        "pre-commit:\n  commands:\n    lint:\n      run: lint\n{}\n",
        ah_block().start_marker
    );
    let fixture = config_fixture(&config);
    let err = lefthook::ensure_wired(
        fixture.root(),
        lefthook::Stage::PreCommit,
        &ah_block(),
        ah_content(),
    )
    .unwrap_err();
    assert!(
        matches!(err, GitHooksError::UnbalancedLefthookMarkers { .. }),
        "got: {err}"
    );
    assert_eq!(
        wired_config(fixture.root()),
        config,
        "config must not be modified"
    );
}

/// genesis-5sm: children at indent 6 but `commands:` at indent 2 —
/// splicing after that key would nest the block at the wrong level;
/// same UnanchorableLefthookConfig refusal as ensure_command_wired.
#[test]
fn ensure_wired_refuses_indent_mismatched_commands_key_unmodified() {
    let config = "pre-commit:\n      parallel: true\n  commands:\n    lint:\n      run: lint\n";
    let fixture = config_fixture(config);
    let err = lefthook::ensure_wired(
        fixture.root(),
        lefthook::Stage::PreCommit,
        &ah_block(),
        ah_content(),
    )
    .unwrap_err();
    assert!(
        matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
        "got: {err}"
    );
    assert!(
        err.to_string().contains("commands key is at indent"),
        "error should explain the refusal: {err}"
    );
    assert_eq!(
        wired_config(fixture.root()),
        config,
        "config must not be modified"
    );
}

#[test]
fn ensure_wired_refuses_bare_html_comment_markers_unmodified() {
    // genesis-5sm: `<!-- AH:START -->` markers (BlockDef::new's default)
    // emitted into the config produce unparseable YAML — validate and
    // refuse; callers declare comment-prefixed markers instead.
    let fixture = config_fixture(basic_config());
    let err = lefthook::ensure_wired(
        fixture.root(),
        lefthook::Stage::PreCommit,
        &crate::managed_block::BlockDef::new("AH"),
        ah_content(),
    )
    .unwrap_err();
    assert!(
        matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
        "got: {err}"
    );
    assert!(
        err.to_string().contains("unparseable YAML"),
        "error should name the problem: {err}"
    );
    assert_eq!(
        wired_config(fixture.root()),
        basic_config(),
        "config must not be modified"
    );
}

#[test]
fn ensure_wired_anchors_past_comment_lines_mentioning_the_stage() {
    // GH#28 item 1: comments mentioning `pre-commit` before the real
    // column-0 key must not trip the contains-refusal (D6) — the
    // stage is anchorable.
    let config = "# pre-commit hooks are managed by ah\npre-commit:\n  commands:\n    test:\n      run: test\n";
    let fixture = config_fixture(config);
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let after = wired_config(fixture.root());
    assert!(after.contains("AH:START"), "block should be wired");
    assert!(
        after.starts_with("# pre-commit hooks are managed by ah\npre-commit:\n  commands:\n# <!--"),
        "block must nest inside commands:, comment preserved\n---\n{after}"
    );
}

#[test]
fn ensure_wired_treats_comment_only_stage_mention_as_absent() {
    // Stage name appears only in a comment: not present (D6 refusal
    // must not fire), so the section is appended.
    let fixture = config_fixture("# no pre-commit stage yet, see docs\nother: key\n");
    wire(fixture.root(), lefthook::Stage::PreCommit);
    assert!(
        wired_config(fixture.root()).contains("AH:START"),
        "block should be wired"
    );
}

#[test]
fn injected_block_is_recognized_by_managed_block_reader() {
    let fixture = config_fixture(basic_config());
    wire(fixture.root(), lefthook::Stage::PreCommit);
    let mut reg = crate::managed_block::BlockRegistry::new();
    reg.register(ah_block());
    let injector = crate::managed_block::BlockInjector::new(reg);
    let path = fixture.root().join("lefthook.yml");
    assert!(injector.has_block(&path, "AH"));
    let read = injector.read_block(&path, "AH").require("value");
    assert!(read.contains(ah_block().start_marker.as_str()));
    assert!(read.contains("run: ah check"));
}

#[path = "wiring/is_wired.rs"]
mod is_wired;
