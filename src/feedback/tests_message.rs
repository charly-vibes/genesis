//! Tests for the captured error-message field (genesis-c9x): the
//! `--from-last-error` body renders the message under "## Error" when
//! present and nothing extra for legacy records without it.

use super::*;

#[test]
fn from_last_error_body_renders_captured_message() {
    let dir = tmp();
    let tool = test_tool("msg-render");
    write_scratch(&tool, 1);

    let args = FeedbackArgs::new("bug", true, true);
    let (_, body) =
        content::issue_title_and_body(&args, &tool).expect("body should build from scratch record");
    let joined = body.join("\n");
    assert!(
        joined.contains("simulated failure"),
        "body should contain captured message, got: {joined}"
    );
}

#[test]
fn from_last_error_body_omits_message_when_absent() {
    let dir = tmp();
    let tool = test_tool("msg-absent");
    let record = ErrorRecord {
        ts: "2026-07-29T12:00:00Z".into(),
        argv: vec![tool.clone(), "check".into()],
        exit: 1,
        footer: None,
        kind: "error".into(),
        message: None,
    };
    scratch::write_scratch_best_effort(&tool, &record);
    let args = FeedbackArgs::new("bug", true, true);
    let (_, body) =
        content::issue_title_and_body(&args, &tool).expect("body should build from scratch record");
    assert!(
        !body.join("\n").contains("```"),
        "no message code block should render"
    );
}
