//! Unit tests for `guide` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use crate::envelope::EnvelopeKind;
use crate::suggestions::Suggestion;

// -- Verbosity ---------------------------------------------------------

#[test]
fn test_verbosity_from_u8() {
    assert_eq!(Verbosity::from(0u8), Verbosity::Quiet);
    assert_eq!(Verbosity::from(1u8), Verbosity::Normal);
    assert_eq!(Verbosity::from(2u8), Verbosity::Verbose);
    assert_eq!(Verbosity::from(3u8), Verbosity::Debug);
    assert_eq!(Verbosity::from(4u8), Verbosity::Debug);
    assert_eq!(Verbosity::from(255u8), Verbosity::Debug);
}

#[test]
fn test_verbosity_should_show() {
    let quiet = Verbosity::Quiet;
    assert!(!quiet.should_show(1));
    assert!(!quiet.should_show(2));

    let normal = Verbosity::Normal;
    assert!(normal.should_show(0));
    assert!(normal.should_show(1));
    assert!(!normal.should_show(2));

    let verbose = Verbosity::Verbose;
    assert!(verbose.should_show(0));
    assert!(verbose.should_show(1));
    assert!(verbose.should_show(2));
    assert!(!verbose.should_show(3));

    let debug = Verbosity::Debug;
    assert!(debug.should_show(0));
    assert!(debug.should_show(1));
    assert!(debug.should_show(2));
    assert!(debug.should_show(3));
}

#[test]
fn test_verbosity_as_u8() {
    assert_eq!(Verbosity::Quiet.as_u8(), 0);
    assert_eq!(Verbosity::Normal.as_u8(), 1);
    assert_eq!(Verbosity::Verbose.as_u8(), 2);
    assert_eq!(Verbosity::Debug.as_u8(), 3);
}

#[test]
fn test_verbosity_display() {
    assert_eq!(format!("{}", Verbosity::Quiet), "quiet");
    assert_eq!(format!("{}", Verbosity::Normal), "normal");
    assert_eq!(format!("{}", Verbosity::Verbose), "verbose");
    assert_eq!(format!("{}", Verbosity::Debug), "debug");
}

// -- Output construction -----------------------------------------------

#[test]
fn test_output_success() {
    let output: Output<i32> = Output::success(42);
    assert_eq!(output.data, 42);
    assert!(!output.is_error);
    assert!(output.next_step.is_none());
    assert!(output.warnings.is_empty());
    assert_eq!(output.verbosity, 1);
}

#[test]
fn test_output_failure() {
    let output: Output<String> = Output::failure("something broke");
    assert!(output.is_error);
    assert_eq!(output.warnings, vec!["something broke"]);
    assert!(output.next_step.is_none());
}

#[test]
fn test_output_with_next_step() {
    let output: Output<&str> = Output::success("done").with_next_step("run doctor");
    assert!(output.next_step.is_some());
    let suggestion = output.next_step.as_ref().require("value");
    let footer = suggestion.footer().require("value");
    assert!(footer.contains("run doctor"));
}

// genesis-r13: to_envelope must route failures to an envelope whose
// ok field agrees with the error kind.
#[test]
fn test_to_envelope_error_output_carries_ok_false() {
    let out: Output<&str> = Output::failure("bad path");
    let env = out.to_envelope("t/1.0");
    assert_eq!(env.envelope_kind, EnvelopeKind::Error);
    assert!(!env.ok, "failed Output must serialize ok:false");
    let json = serde_json::to_string(&env).require("value");
    assert!(json.contains("\"ok\":false"), "got: {json}");
}

// genesis-r13: the JSON hint command must be runnable — footer()
// embeds the human '→ Run: ' prefix; to_envelope must emit the bare
// command instead.
#[test]
fn test_to_envelope_hint_command_is_runnable() {
    let out: Output<&str> = Output::success("ok").with_next_step("run: specodelic graph");
    let env = out.to_envelope("t/1.0");
    let hints = env.hints.require("hints present");
    assert_eq!(hints[0].command, "specodelic graph");
}

#[test]
fn test_output_with_warning() {
    let output: Output<&str> = Output::success("ok").with_warning("check config");
    assert_eq!(output.warnings, vec!["check config"]);
}

#[test]
fn test_output_with_verbosity() {
    let output: Output<&str> = Output::success("data").with_verbosity(2);
    assert_eq!(output.verbosity, 2);
}

#[test]
fn test_output_fluent_chaining() {
    let output: Output<&str> = Output::success("result")
        .with_next_step("next")
        .with_warning("warn1")
        .with_warning("warn2")
        .with_verbosity(1);

    assert_eq!(output.data, "result");
    assert!(output.next_step.is_some());
    assert_eq!(output.warnings.len(), 2);
    assert_eq!(output.verbosity, 1);
}

// -- Output::print -----------------------------------------------------

#[test]
fn test_print_success_at_normal_verbosity() {
    let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    output
        .print(Verbosity::Normal, &mut stdout, &mut stderr)
        .require("value");

    let stdout = String::from_utf8(stdout).require("value");
    let stderr = String::from_utf8(stderr).require("value");

    assert!(stdout.contains("hello"));
    assert!(stderr.contains("run doctor"));
}

#[test]
fn test_print_success_at_quiet_verbosity() {
    let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    output
        .print(Verbosity::Quiet, &mut stdout, &mut stderr)
        .require("value");

    let stdout = String::from_utf8(stdout).require("value");
    let stderr = String::from_utf8(stderr).require("value");

    // Quiet shows nothing
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_print_failure_always_shows() {
    let output: Output<String> = Output::failure("critical error");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    output
        .print(Verbosity::Quiet, &mut stdout, &mut stderr)
        .require("value");

    let stderr = String::from_utf8(stderr).require("value");
    assert!(stderr.contains("critical error"));
}

#[test]
fn test_print_warnings_only_at_verbose() {
    let output: Output<&str> = Output::success("data").with_warning("a warning");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    // Normal verbosity -- no warnings
    output
        .print(Verbosity::Normal, &mut stdout, &mut stderr)
        .require("value");
    let stderr_normal = String::from_utf8(stderr.clone()).require("value");
    assert!(!stderr_normal.contains("a warning"));

    // Verbose verbosity -- warnings shown
    let mut stderr = Vec::new();
    output
        .print(Verbosity::Verbose, &mut stdout, &mut stderr)
        .require("value");
    let stderr_verbose = String::from_utf8(stderr).require("value");
    assert!(stderr_verbose.contains("a warning"));
}

#[test]
fn test_print_verbosity_threshold() {
    let output: Output<&str> = Output::success("secret").with_verbosity(2);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    // Normal (1) is below threshold (2) -- data not shown
    output
        .print(Verbosity::Normal, &mut stdout, &mut stderr)
        .require("value");
    let stdout_normal = String::from_utf8(stdout).require("value");
    assert!(!stdout_normal.contains("secret"));

    // Verbose (2) meets threshold -- data shown
    let mut stdout = Vec::new();
    output
        .print(Verbosity::Verbose, &mut stdout, &mut stderr)
        .require("value");
    let stdout_verbose = String::from_utf8(stdout).require("value");
    assert!(stdout_verbose.contains("secret"));
}

// -- Output::to_envelope -----------------------------------------------

#[test]
fn test_to_envelope_success() {
    let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
    let envelope = output.to_envelope("my-tool/1.0.0");

    assert!(envelope.ok);
    assert_eq!(envelope.data, &"hello");
    assert_eq!(envelope.cli_version, "my-tool/1.0.0");
}

#[test]
fn test_to_envelope_includes_warnings() {
    let output: Output<&str> = Output::success("data").with_warning("warn1");
    let envelope = output.to_envelope("my-tool/1.0.0");

    assert!(!envelope.warnings.is_empty());
    assert_eq!(envelope.warnings[0].message, "warn1");
}

#[test]
fn test_to_envelope_includes_hints() {
    let output: Output<&str> = Output::success("data").with_next_step("run doctor");
    let envelope = output.to_envelope("my-tool/1.0.0");

    assert!(envelope.hints.is_some());
    let hints = envelope.hints.require("value");
    assert!(!hints.is_empty());
}

#[test]
fn test_to_envelope_serializes_to_json() {
    let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
    let envelope = output.to_envelope("my-tool/1.0.0");

    let json = serde_json::to_string(&envelope).require("value");
    assert!(json.contains("hello"));
    assert!(json.contains("run doctor"));
    assert!(json.contains("my-tool/1.0.0"));
}

// -- ErrorSink ---------------------------------------------------------

#[test]
fn test_error_sink_defaults() {
    let sink = ErrorSink::new("test-tool");
    assert_eq!(sink.tool_name, "test-tool");
    assert!(sink.scratch);
    assert!(sink.suggest);
    assert!(!sink.context);
    assert_eq!(sink.feedback_subcommand.as_deref(), Some("feedback"));
}

#[test]
fn test_error_sink_fluent_builders() {
    let sink = ErrorSink::new("tool")
        .with_verbosity(Verbosity::Quiet)
        .with_scratch(false)
        .with_suggest(false)
        .with_feedback(None);

    assert_eq!(sink.verbosity, Verbosity::Quiet);
    assert!(!sink.scratch);
    assert!(!sink.suggest);
    assert!(sink.feedback_subcommand.is_none());
}

#[test]
fn test_error_sink_handle_prints_error() {
    let sink = ErrorSink::new("test-tool").with_scratch(false);
    let mut stderr = Vec::new();
    let err = std::io::Error::other("something broke");
    sink.handle(&err, &mut stderr);
    let stderr = String::from_utf8(stderr).require("value");
    assert!(stderr.contains("test-tool"));
    assert!(stderr.contains("something broke"));
    assert!(stderr.contains("Run:"));
}

#[test]
fn test_error_sink_handle_without_suggest() {
    let sink = ErrorSink::new("test-tool")
        .with_scratch(false)
        .with_suggest(false);
    let mut stderr = Vec::new();
    let err = std::io::Error::other("fail");
    sink.handle(&err, &mut stderr);
    let stderr = String::from_utf8(stderr).require("value");
    assert!(stderr.contains("fail"));
    // No suggestion footer
    assert!(!stderr.contains("Run:"));
}

#[test]
fn test_error_sink_handle_with_footer() {
    let sink = ErrorSink::new("tool").with_scratch(false);
    let mut stderr = Vec::new();
    let err = std::io::Error::other("err");
    let suggestion = Suggestion::fix("custom fix");
    sink.handle_with_footer(&err, &suggestion, &mut stderr);
    let stderr = String::from_utf8(stderr).require("value");
    assert!(stderr.contains("custom fix"));
}

// -- GuideBuilder & Guide ----------------------------------------------
