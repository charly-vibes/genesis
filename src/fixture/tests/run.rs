//! Tests for [`Fixture::run`], [`CommandOutput`], and `FixtureError`
//! display plumbing.

// pretender: role=test
use crate::fixture::{CommandOutput, Fixture, FixtureError};
use crate::testutil::TestUnwrap;

#[cfg(unix)]
#[test]
fn test_run_echo_command() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["echo", "hello world"]).require("run");
    assert!(output.success());
    assert!(output.stdout.contains("hello world"));
}

#[cfg(unix)]
#[test]
fn test_run_failure_exit_code() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["sh", "-c", "exit 42"]).require("run");
    assert!(!output.success());
    assert_eq!(output.exit_code, 42);
    assert!(output.signal.is_none());
}

#[cfg(unix)]
#[test]
fn test_run_with_stderr() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture
        .run(&["sh", "-c", "echo 'error msg' >&2"])
        .require("run");
    assert!(output.success());
    assert!(output.stderr.contains("error msg"));
}

#[cfg(unix)]
#[test]
fn test_run_json_output() {
    let fixture = Fixture::new().build().require("build");
    let output = fixture.run(&["echo", "{\"key\":\"value\"}"]).require("run");
    let parsed: Option<serde_json::Value> = output.json();
    assert!(parsed.is_some());
    assert_eq!(parsed.require("value")["key"], "value");
}

#[cfg(unix)]
#[test]
fn test_run_in_fixture_directory() {
    let fixture = Fixture::new()
        .with_file("test.txt", "fixture content")
        .build()
        .require("build");
    let output = fixture.run(&["cat", "test.txt"]).require("run");
    assert!(output.success());
    assert!(output.stdout.contains("fixture content"));
}

#[test]
fn test_run_empty_args_returns_err() {
    let fixture = Fixture::new().build().require("build");
    let result = fixture.run(&[]);
    assert!(matches!(result, Err(FixtureError::EmptyCommand)));
}

#[test]
fn test_run_non_existent_command_returns_err() {
    let fixture = Fixture::new().build().require("build");
    let result = fixture.run(&["nonexistent_command_xyz123"]);
    assert!(matches!(result, Err(FixtureError::Spawn { .. })));
}

#[test]
fn test_command_output_success() {
    let ok = CommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
        signal: None,
    };
    assert!(ok.success());

    let fail = CommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 1,
        signal: None,
    };
    assert!(!fail.success());
}

#[test]
fn test_command_output_json_invalid() {
    let output = CommandOutput {
        stdout: "not json".into(),
        stderr: String::new(),
        exit_code: 0,
        signal: None,
    };
    let result: Option<serde_json::Value> = output.json();
    assert!(result.is_none());
}

#[test]
fn test_fixture_error_display_non_empty() {
    let e = FixtureError::EmptyCommand;
    assert!(!e.to_string().is_empty());
}
