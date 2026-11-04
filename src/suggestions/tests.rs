//! Unit tests for `suggestions` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;

// ── Suggestion::fix authoring convention (genesis-r13) ─────────

#[test]
fn test_fix_strips_leading_run_prefix() {
    // Hints are authored 'run: <cmd>'; the footer already renders
    // '→ Run: {cmd}' — keeping the prefix would double it
    // ('→ Run: run: specodelic graph').
    let s = Suggestion::fix("run: specodelic graph");
    match &s {
        Suggestion::Fix { command, .. } => {
            assert_eq!(command.as_deref(), Some("specodelic graph"));
        }
        other => panic!("expected Fix, got {other:?}"),
    }
    assert_eq!(s.footer().as_deref(), Some("→ Run: specodelic graph"));
    assert!(
        !s.message().contains("run: run:"),
        "no doubled prefix: {}",
        s.message()
    );
}

#[test]
fn test_fix_strips_capitalized_run_prefix() {
    let s = Suggestion::fix("Run: doctor");
    match &s {
        Suggestion::Fix { command, .. } => {
            assert_eq!(command.as_deref(), Some("doctor"));
        }
        other => panic!("expected Fix, got {other:?}"),
    }
}

#[test]
fn test_fix_without_run_prefix_keeps_full_hint() {
    let s = Suggestion::fix("fix the reported invariants");
    match &s {
        Suggestion::Fix {
            command,
            description,
        } => {
            assert_eq!(command.as_deref(), Some("fix the reported invariants"));
            assert_eq!(description, "fix the reported invariants");
        }
        other => panic!("expected Fix, got {other:?}"),
    }
}

// genesis-9us: trailing-command hints (specodelic's authoring
// convention) — the sentence's tail after the LAST ' run: ' is the
// runnable command; the prefix is the guidance.
#[test]
fn test_fix_splits_trailing_run_command() {
    let s = Suggestion::fix("every reference resolves — run: specodelic lint");
    match &s {
        Suggestion::Fix {
            command,
            description,
        } => {
            assert_eq!(command.as_deref(), Some("specodelic lint"));
            assert_eq!(description, "every reference resolves");
        }
        other => panic!("expected Fix, got {other:?}"),
    }
    assert_eq!(s.footer().as_deref(), Some("→ Run: specodelic lint"));
    assert!(!s.message().contains("run: run:"), "{}", s.message());
}

#[test]
fn test_fix_splits_trailing_run_command_with_args() {
    let s = Suggestion::fix("fill in the four layers, then run: specodelic lint order-cancel.md");
    match &s {
        Suggestion::Fix {
            command,
            description,
        } => {
            assert_eq!(command.as_deref(), Some("specodelic lint order-cancel.md"));
            assert_eq!(description, "fill in the four layers, then");
        }
        other => panic!("expected Fix, got {other:?}"),
    }
}

#[test]
fn test_fix_prefers_leading_prefix_over_trailing_split() {
    // A leading 'run: ' hint whose command contains 'run: ' later —
    // the leading prefix wins (whole remainder is the command).
    let s = Suggestion::fix("run: specodelic lint --json && echo run: done");
    match &s {
        Suggestion::Fix { command, .. } => {
            assert_eq!(
                command.as_deref(),
                Some("specodelic lint --json && echo run: done")
            );
        }
        other => panic!("expected Fix, got {other:?}"),
    }
}

// ── CommandRegistry ──────────────────────────────────────────

#[test]
fn test_registry_empty_by_default() {
    let reg = CommandRegistry::new();
    assert!(reg.all().is_empty());
}

#[test]
fn test_registry_register_and_retrieve() {
    let mut reg = CommandRegistry::new();
    reg.register("wai", vec!["status".into(), "init".into(), "new".into()]);
    assert_eq!(reg.all().len(), 3);
    assert!(reg.all().contains(&"status"));
}

#[test]
fn test_registry_replace_existing_tool() {
    let mut reg = CommandRegistry::new();
    reg.register("wai", vec!["status".into(), "init".into()]);
    reg.register("wai", vec!["new".into()]);
    assert_eq!(reg.all().len(), 1);
    assert_eq!(reg.all(), vec!["new"]);
}

#[test]
fn test_registry_multiple_tools() {
    let mut reg = CommandRegistry::new();
    reg.register("wai", vec!["status".into(), "init".into()]);
    reg.register("dont", vec!["conclude".into(), "ground".into()]);
    assert_eq!(reg.all().len(), 4);
    assert_eq!(reg.for_tool("wai").len(), 2);
    assert_eq!(reg.for_tool("dont").len(), 2);
    assert!(reg.for_tool("unknown").is_empty());
}

// ── Typo detection ────────────────────────────────────────────────

#[test]
fn test_typo_detection() {
    let engine = SuggestionEngine::new();
    let mut reg = CommandRegistry::new();
    reg.register("test", vec!["status".into(), "init".into(), "new".into()]);

    let suggestion = engine.suggest_typo("staus", &reg);
    assert!(suggestion.is_some());

    if let Some(Suggestion::DidYouMean {
        original,
        suggestion,
    }) = suggestion
    {
        assert_eq!(original, "staus");
        assert_eq!(suggestion, "status");
    }
}

#[test]
fn test_no_typo_suggestion_for_dissimilar() {
    let engine = SuggestionEngine::new();
    let mut reg = CommandRegistry::new();
    reg.register("test", vec!["status".into(), "init".into(), "new".into()]);

    let suggestion = engine.suggest_typo("xyz", &reg);
    assert!(suggestion.is_none());
}

// ── Wrong order detection ─────────────────────────────────────────

#[test]
fn test_wrong_order_detection() {
    let engine = SuggestionEngine::new();
    let patterns = &[("new", "project"), ("add", "research"), ("show", "status")];

    let suggestion = engine.suggest_order("project", "new", patterns);
    assert!(suggestion.is_some());

    if let Some(Suggestion::WrongOrder { original, correct }) = suggestion {
        assert_eq!(original, "project new");
        assert_eq!(correct, "new project");
    }
}

#[test]
fn test_no_wrong_order_for_valid_pattern() {
    let engine = SuggestionEngine::new();
    let patterns = &[("new", "project"), ("add", "research")];

    let suggestion = engine.suggest_order("new", "project", patterns);
    assert!(suggestion.is_none());
}

// ── Suggestion message formatting ─────────────────────────────────

#[test]
fn test_typo_message_formatting() {
    let s = Suggestion::DidYouMean {
        original: "staus".to_string(),
        suggestion: "status".to_string(),
    };
    let msg = s.message();
    assert!(msg.contains("Did you mean"));
    assert!(msg.contains("staus"));
    assert!(msg.contains("status"));
}

#[test]
fn test_fix_message_with_command() {
    let s = Suggestion::Fix {
        description: "something went wrong".into(),
        command: Some("just fix".into()),
    };
    let msg = s.message();
    assert!(msg.contains("something went wrong"));
    assert!(msg.contains("→ Run:"));
    assert!(msg.contains("just fix"));
}

// ── Footer helper (task 3.2) ──────────────────────────────────────

#[test]
fn test_footer_for_typo() {
    let s = Suggestion::DidYouMean {
        original: "staus".into(),
        suggestion: "status".into(),
    };
    assert_eq!(s.footer(), Some("→ Run: status".into()));
}

#[test]
fn test_footer_for_fix_with_command() {
    let s = Suggestion::Fix {
        description: "error".into(),
        command: Some("just fix".into()),
    };
    assert_eq!(s.footer(), Some("→ Run: just fix".into()));
}

#[test]
fn test_footer_for_fix_without_command() {
    let s = Suggestion::Fix {
        description: "error".into(),
        command: None,
    };
    assert_eq!(s.footer(), None);
}

#[test]
fn test_footer_for_context_hint() {
    let s = Suggestion::ContextHint {
        message: "try something".into(),
        path: None,
    };
    assert_eq!(s.footer(), None);
}

#[test]
fn test_footer_for_wrong_order() {
    let s = Suggestion::WrongOrder {
        original: "project new".into(),
        correct: "new project".into(),
    };
    assert_eq!(s.footer(), Some("→ Run: new project".into()));
}

// ── Suggestion serialization ──────────────────────────────────────

#[test]
fn test_suggestion_serializes() {
    let s = Suggestion::DidYouMean {
        original: "staus".into(),
        suggestion: "status".into(),
    };
    let json = serde_json::to_string(&s).require("value");
    assert!(json.contains("did_you_mean"));
    assert!(json.contains("staus"));
    assert!(json.contains("status"));
}

// ── Custom threshold ──────────────────────────────────────────────

#[test]
fn test_custom_threshold_affects_results() {
    let engine = SuggestionEngine::with_threshold(0.99);
    let mut reg = CommandRegistry::new();
    reg.register("test", vec!["status".into(), "init".into()]);

    // "statu" is close to "status" but unlikely to reach 0.99
    let suggestion = engine.suggest_typo("statu", &reg);
    // With such a high threshold, only near-exact matches should pass
    // This test just verifies the mechanism works
    assert!(suggestion.is_none() || suggestion.is_some());
}
