//! Unit tests for `status` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;

// ── StatusLevel ───────────────────────────────────────────────────

#[test]
fn test_status_level_is_issue() {
    assert!(!StatusLevel::Healthy.is_issue());
    assert!(StatusLevel::Warning.is_issue());
    assert!(StatusLevel::Error.is_issue());
}

#[test]
fn test_status_level_is_error() {
    assert!(!StatusLevel::Healthy.is_error());
    assert!(!StatusLevel::Warning.is_error());
    assert!(StatusLevel::Error.is_error());
}

// ── StatusItem ────────────────────────────────────────────────────

#[test]
fn test_status_item_healthy() {
    let item = StatusItem::healthy("config", "present");
    assert_eq!(item.level, StatusLevel::Healthy);
}

#[test]
fn test_status_item_warning() {
    let item = StatusItem::warning("config", "deprecated");
    assert_eq!(item.level, StatusLevel::Warning);
}

#[test]
fn test_status_item_error() {
    let item = StatusItem::error("state", "missing");
    assert_eq!(item.level, StatusLevel::Error);
}

// ── StatusSection ─────────────────────────────────────────────────

#[test]
fn test_section_healthy_auto_level() {
    let section = StatusSection::healthy("tool", "ok");
    assert_eq!(section.level, StatusLevel::Healthy);
}

#[test]
fn test_section_with_items_derives_level() {
    let items = vec![StatusItem::warning("x", "deprecated")];
    let section = StatusSection::with_items("tool", "summary", items);
    assert_eq!(section.level, StatusLevel::Warning);
}

#[test]
fn test_section_with_items_error_takes_priority() {
    let items = vec![
        StatusItem::warning("x", "deprecated"),
        StatusItem::error("y", "missing"),
    ];
    let section = StatusSection::with_items("tool", "summary", items);
    assert_eq!(section.level, StatusLevel::Error);
}

#[test]
fn test_section_with_suggestion() {
    let section = StatusSection::healthy("tool", "ok").with_suggestion("run doctor");
    assert_eq!(section.suggestions.len(), 1);
    assert_eq!(section.suggestions[0], "run doctor");
}

// ── MultiToolStatus ───────────────────────────────────────────────

#[test]
fn test_status_summary_all_healthy() {
    let report = MultiToolStatus {
        sections: vec![
            StatusSection::healthy("a", "ok"),
            StatusSection::healthy("b", "ok"),
        ],
    };
    let summary = report.summary();
    assert_eq!(summary.total, 2);
    assert_eq!(summary.healthy, 2);
    assert!(summary.is_healthy());
}

#[test]
fn test_status_summary_mixed() {
    let report = MultiToolStatus {
        sections: vec![
            StatusSection::healthy("a", "ok"),
            StatusSection {
                tool: "b".into(),
                level: StatusLevel::Warning,
                summary: "warn".into(),
                items: vec![],
                suggestions: vec![],
            },
            StatusSection {
                tool: "c".into(),
                level: StatusLevel::Error,
                summary: "fail".into(),
                items: vec![],
                suggestions: vec![],
            },
        ],
    };
    let summary = report.summary();
    assert_eq!(summary.total, 3);
    assert_eq!(summary.healthy, 1);
    assert_eq!(summary.warnings, 1);
    assert_eq!(summary.errors, 1);
    assert!(!summary.is_healthy());
}

#[test]
fn test_is_healthy() {
    let healthy = MultiToolStatus {
        sections: vec![StatusSection::healthy("a", "ok")],
    };
    assert!(healthy.is_healthy());

    let warning = MultiToolStatus {
        sections: vec![StatusSection {
            tool: "b".into(),
            level: StatusLevel::Warning,
            summary: "w".into(),
            items: vec![],
            suggestions: vec![],
        }],
    };
    assert!(!warning.is_healthy());
}

#[test]
fn test_has_errors() {
    let ok = MultiToolStatus {
        sections: vec![StatusSection::healthy("a", "ok")],
    };
    assert!(!ok.has_errors());

    let err = MultiToolStatus {
        sections: vec![StatusSection {
            tool: "b".into(),
            level: StatusLevel::Error,
            summary: "e".into(),
            items: vec![],
            suggestions: vec![],
        }],
    };
    assert!(err.has_errors());
}

#[test]
fn test_all_suggestions() {
    let report = MultiToolStatus {
        sections: vec![
            StatusSection::healthy("a", "ok").with_suggestion("do x"),
            StatusSection {
                tool: "b".into(),
                level: StatusLevel::Warning,
                summary: "w".into(),
                items: vec![],
                suggestions: vec!["do y".into(), "do z".into()],
            },
        ],
    };
    let suggestions = report.all_suggestions();
    assert_eq!(suggestions.len(), 3);
}

#[test]
fn test_to_envelope() {
    let report = MultiToolStatus {
        sections: vec![
            StatusSection::healthy("a", "ok"),
            StatusSection {
                tool: "b".into(),
                level: StatusLevel::Warning,
                summary: "deprecated field".to_string(),
                items: vec![],
                suggestions: vec!["run doctor".into()],
            },
        ],
    };
    let envelope = report.to_envelope("my-tool/1.0.0");
    assert!(envelope.ok);
    assert_eq!(envelope.cli_version, "my-tool/1.0.0");
}

#[test]
fn test_to_envelope_serializes_json() {
    let report = MultiToolStatus {
        sections: vec![StatusSection::healthy("tool", "ok")],
    };
    let envelope = report.to_envelope("my-tool/1.0.0");
    let json = serde_json::to_string(&envelope).require("value");
    assert!(json.contains("health"));
    assert!(json.contains("my-tool/1.0.0"));
    // No warnings expected since sections are healthy
}
