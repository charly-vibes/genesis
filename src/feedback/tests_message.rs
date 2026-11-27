//! Tests for the captured error-message field (genesis-c9x): the
//! `--from-last-error` body renders the message under "## Error" when
//! present and nothing extra for legacy records without it.

use super::*;

#[test]
fn from_last_error_body_renders_captured_message() {
    let _dir = tmp();
    let tool = test_tool("msg-render");
    write_scratch(&tool, 1);

    let args = FeedbackArgs::new("bug", true, true);
    let issue =
        content::issue_title_and_body(&args, &tool).expect("body should build from scratch record");
    let joined = issue.body_parts.join("\n");
    assert!(
        joined.contains("simulated failure"),
        "body should contain captured message, got: {joined}"
    );
}

#[test]
fn from_last_error_body_omits_message_when_absent() {
    let _dir = tmp();
    let tool = test_tool("msg-absent");
    let record = ErrorRecord::new(
        "2026-07-29T12:00:00Z",
        vec![tool.clone(), "check".into()],
        1,
        "error",
    );
    scratch::write_scratch_best_effort(&tool, &record);
    let args = FeedbackArgs::new("bug", true, true);
    let issue =
        content::issue_title_and_body(&args, &tool).expect("body should build from scratch record");
    assert!(
        !issue.body_parts.join("\n").contains("```"),
        "no message code block should render"
    );
}

// genesis-ase: fence injection and length cap under `## Error`.

/// Strip the fence from a rendered block so the original message can be
/// compared against the fenced interior.
///
/// Roundtrip contract: the whole message — including embedded ``` runs —
/// renders inside the fence, which opens after the fence line, and no
/// line of the message equals the fence.
fn unfenced<'a>(block: &'a str, fence: &str) -> &'a str {
    block
        .trim_start_matches(fence)
        .trim_start_matches('\n')
        .trim_end_matches(format!("{fence}\n\n").as_str())
        .trim_end_matches(fence)
        .trim_end_matches('\n')
}

#[test]
fn fenced_code_block_swallows_embedded_backtick_fence() {
    let message = "before\n```rust\nlet x = 1;\n```\nafter";
    let block = content::fenced_code_block(message);
    let fence = block
        .lines()
        .next()
        .expect("fence line must exist")
        .to_string();
    assert!(
        fence.starts_with("````"),
        "fence must be longer than 3: {block}"
    );
    assert!(block.contains("before"));
    assert!(block.contains("```rust"));
    assert!(block.contains("after"));
    assert_eq!(unfenced(&block, fence.as_str()), message);
}

#[test]
fn fenced_code_block_keeps_fence_sized_to_longest_run() {
    let block = content::fenced_code_block("a `` double run");
    assert!(block.starts_with("```\na `` double run\n```"));
    let block = content::fenced_code_block("plain message");
    assert!(block.starts_with("```\nplain message\n```"));
}

#[test]
fn fenced_code_block_truncates_over_long_message_with_marker() {
    let message = "x".repeat(5000);
    let block = content::fenced_code_block(&message);
    assert!(
        block.contains("message truncated"),
        "marker missing: {block}"
    );
    let inner_len = block
        .lines()
        .filter(|l| !l.starts_with("```"))
        .map(|l| l.chars().count())
        .sum::<usize>();
    assert!(
        inner_len <= 5000,
        "rendered message should be capped near 4000 chars, got {inner_len}"
    );
    // A short message is never truncated.
    let block = content::fenced_code_block("short");
    assert!(!block.contains("truncated"));
}
