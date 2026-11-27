//! Stdin title/body partitioning and title-override builder tests
//! (genesis-gle, genesis-og6).

use super::content::{apply_title_override, stdin_title_and_body};
use super::*;

// --- stdin title/body partitioning (genesis-gle, genesis-og6) ---

#[test]
fn stdin_single_line_keeps_generic_title() {
    let (title, body) = stdin_title_and_body("bug", "summary line");
    assert_eq!(title, "[bug] feedback report");
    assert!(body.contains("summary line"), "body must carry the input");
}

#[test]
fn stdin_trailing_newline_is_still_single_line() {
    // `echo "x" | tool feedback bug` produces "x\n" — must keep the
    // generic title, not treat the trailing newline as a second line.
    let (title, body) = stdin_title_and_body("bug", "summary line\n");
    assert_eq!(title, "[bug] feedback report");
    assert!(body.contains("summary line"));
}

#[test]
fn stdin_multiline_promotes_first_line_to_title() {
    let input = "Bare row-id references fail when intent id has a dot\n\nThe resolution algorithm is undocumented. Details follow.\n";
    let (title, body) = stdin_title_and_body("bug", input);
    assert_eq!(
        title,
        "[bug] Bare row-id references fail when intent id has a dot"
    );
    assert!(body.contains("The resolution algorithm is undocumented."));
    assert!(
        !body.contains("Bare row-id references"),
        "first line must not be duplicated into the body"
    );
    assert!(body.starts_with("## Description"));
}

#[test]
fn stdin_multiline_blank_lines_around_single_line_is_still_single() {
    // A title line followed by blank lines trims to one line — the
    // echo-compat rule wins over title promotion.
    let (title, body) = stdin_title_and_body("feature", "add a flag\n\n");
    assert_eq!(title, "[feature] feedback report");
    assert!(body.contains("add a flag"));
}

#[test]
fn stdin_multiline_title_plus_body_omits_first_line_from_body() {
    let (title, body) = stdin_title_and_body("feature", "add a flag\n\nbody text\n");
    assert_eq!(title, "[feature] add a flag");
    assert!(body.contains("body text"));
    assert!(!body.contains("add a flag"));
}

#[test]
fn title_override_wins_over_derived_and_first_line() {
    assert_eq!(
        apply_title_override("bug", "[bug] feedback report", Some("My title")),
        "[bug] My title"
    );
    assert_eq!(
        apply_title_override(
            "bug",
            "[bug] auto-reported error: ah check",
            Some("My title")
        ),
        "[bug] My title"
    );
    assert_eq!(
        apply_title_override("bug", "[bug] feedback report", None),
        "[bug] feedback report",
        "no override keeps the derived title"
    );
}

#[test]
fn with_title_builder_sets_override() {
    let args = FeedbackArgs::new("bug", true, false).with_title("My title");
    assert_eq!(args.title.as_deref(), Some("My title"));
    // ::new keeps the 3-arg signature — no override by default.
    let plain = FeedbackArgs::new("bug", true, false);
    assert_eq!(plain.title, None);
}
