//! `DoctorReport` construction, envelope, and serialization tests.

use super::*;
use crate::envelope::EnvelopeKind;

#[test]
fn test_report_from_checks() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::fail("b", "d", "fail", None),
    ];
    let report = DoctorReport::new("test-tool", checks);
    assert_eq!(report.tool, "test-tool");
    assert!(!report.is_healthy());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn test_report_exit_code_healthy() {
    let checks = vec![CheckEntry::pass("a", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn test_report_to_envelope() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::fail("b", "d", "missing config", Some("tool init".into())),
    ];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    assert!(envelope.ok);
    assert_eq!(envelope.envelope_kind, EnvelopeKind::Doctor);
    assert_eq!(envelope.cli_version, "my-tool/1.0.0");
}

#[test]
fn test_report_to_envelope_serializes_json() {
    let checks = vec![CheckEntry::pass("a", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    let json = serde_json::to_string(&envelope).require("value");
    assert!(json.contains("doctor"));
    assert!(json.contains("test"));
    assert!(json.contains("my-tool/1.0.0"));
}

#[test]
fn test_report_json_serialization() {
    let checks = vec![
        CheckEntry::pass("pass", "d", "ok"),
        CheckEntry::warn("warn", "d", "caution"),
        CheckEntry::fail("fail", "d", "broken", Some("fix".into())),
    ];
    let report = DoctorReport::new("test", checks);
    let json = serde_json::to_string_pretty(&report).require("value");
    assert!(json.contains("\"pass\""));
    assert!(json.contains("\"warn\""));
    assert!(json.contains("\"fail\""));
    assert!(json.contains("\"broken\""));
    assert!(json.contains("\"fix\""));
}

#[test]
fn test_report_json_omits_fix_when_none() {
    let checks = vec![CheckEntry::pass("pass", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    let json = serde_json::to_string(&report).require("value");
    assert!(!json.contains("\"fix\""));
}

#[test]
fn test_report_to_envelope_has_hints_from_fix_commands() {
    let checks = vec![
        CheckEntry::fail("cfg", "d", "missing", Some("tool init".into())),
        CheckEntry::pass("ok", "d", "good"),
    ];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    let hints = envelope.hints.unwrap_or_default();
    assert!(!hints.is_empty());
    assert_eq!(hints[0].command, "tool init");
}

#[test]
fn test_empty_report_to_envelope_has_no_hints() {
    let checks = vec![CheckEntry::pass("ok", "d", "good")];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    assert!(envelope.hints.unwrap_or_default().is_empty());
}
