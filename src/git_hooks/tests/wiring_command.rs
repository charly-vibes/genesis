//! `ensure_command_wired` tests (genesis-au8).

use super::*;
use crate::managed_block::BlockDef;

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
fn ensure_command_wired_placement_cases() {
    // Table-driven: config → expected file after wiring. Covers the
    // in-mapping insert, per-config entry-indent inference (+4 with
    // 4-space children), empty-mapping default (+2), and the wrapper
    // injected when the stage is empty.
    let four_space = "pre-commit:\n    commands:\n        lint:\n            run: lint\n";
    let empty_mapping =
        "pre-commit:\n  commands:\npre-push:\n  commands:\n    test:\n      run: test\n";
    let empty_stage = "pre-commit:\npre-push:\n  commands:\n    test:\n      run: test\n";
    let cases = vec![
        (
            basic_config().to_string(),
            format!(
                "pre-commit:\n  commands:\n{}{}",
                cmd_entry(4),
                &basic_config()["pre-commit:\n  commands:\n".len()..]
            ),
        ),
        (
            four_space.to_string(),
            format!(
                "pre-commit:\n    commands:\n{}        lint:\n            run: lint\n",
                cmd_entry(8)
            ),
        ),
        (
            empty_mapping.to_string(),
            format!(
                "pre-commit:\n  commands:\n{}pre-push:\n  commands:\n    test:\n      run: test\n",
                cmd_entry(4)
            ),
        ),
        (
            empty_stage.to_string(),
            format!(
                "pre-commit:\n{}pre-push:\n  commands:\n    test:\n      run: test\n",
                cmd_wrapper(2)
            ),
        ),
    ];
    for (config, expected) in cases {
        let fixture = config_fixture(&config);
        lefthook::ensure_command_wired(
            fixture.root(),
            lefthook::Stage::PreCommit,
            CMD_NAME,
            CMD_RUN,
            &cmd_block(),
        )
        .require("value");
        let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).require("value");
        assert_eq!(after, expected, "config:\n{config}");
    }
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
    const SOURCE: &str = include_str!("../mod.rs");
    let production = SOURCE
        .split_once("#[cfg(test)]")
        .require("test module marker must exist")
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
