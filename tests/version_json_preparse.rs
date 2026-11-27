//! Regression tests for the `--version --json` pre-parse fast path.
//!
//! genesis-7k6 (EDGE-002): the fast path used to fire whenever `--version`
//! and `--json` both appeared anywhere in argv, silently swallowing
//! everything else on the line — `tool --version --json --bogus` printed the
//! version envelope and exited 0 without the unknown arg ever being seen.
//!
//! Deferring alone is not enough: clap exits 0 on its own `--version` action
//! before validating the rest of the line, so a mixed line must get clap's
//! standard usage error as an error envelope and exit 2. Those
//! process-terminating paths are asserted end to end through the
//! `version-json-probe` harness bin (the documented consumer pattern);
//! the returning branches are asserted in-process via
//! [`maybe_print_version_json_from`].

use clap::Parser;
use genesis::cli::maybe_print_version_json_from;

#[derive(Debug, Parser)]
#[command(name = "my-tool", version)]
struct Cli {
    /// Enable JSON output.
    #[arg(long, short)]
    json: bool,
}

/// Full argv with `argv[0]` set to `my-tool`.
fn argv_tail(tail: &[&str]) -> Vec<String> {
    let mut argv = vec!["my-tool".to_string()];
    argv.extend(tail.iter().map(|a| a.to_string()));
    argv
}

/// `--version --json` alone still takes the fast path unchanged.
#[test]
fn version_json_alone_still_handled() {
    let argv = argv_tail(&["--version", "--json"]);
    let mut out = Vec::new();
    let handled = maybe_print_version_json_from("my-tool", "1.2.3", &argv, &mut out);
    assert!(handled, "bare --version --json must still be intercepted");
    assert!(!out.is_empty(), "version envelope must be printed");
    let parsed: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out).trim()).expect("valid JSON envelope");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert_eq!(parsed["envelope_kind"], serde_json::json!("version"));
}

/// Short forms behave identically.
#[test]
fn short_forms_still_handled() {
    let argv = argv_tail(&["-V", "-j"]);
    let mut out = Vec::new();
    assert!(maybe_print_version_json_from(
        "my-tool", "1.2.3", &argv, &mut out
    ));
    assert!(!out.is_empty());
}

/// Plain `--version` (no `--json`) is left for clap — unchanged.
#[test]
fn plain_version_left_for_clap() {
    let argv = argv_tail(&["--version"]);
    let mut out = Vec::new();
    let handled = maybe_print_version_json_from("my-tool", "1.2.3", &argv, &mut out);
    assert!(!handled, "plain --version is clap's job");
    assert!(out.is_empty(), "fast path must print nothing");
}

/// No version flag at all — unchanged.
#[test]
fn no_version_flag_not_intercepted() {
    let argv = argv_tail(&[]);
    let mut out = Vec::new();
    assert!(!maybe_print_version_json_from(
        "my-tool", "1.2.3", &argv, &mut out
    ));
    assert!(out.is_empty());
}

/// `--help` alongside `--version --json` defers to clap: the display flags'
/// precedence is clap's own job, so the pre-parse neither claims the line
/// nor reports the flag as foreign.
#[test]
fn help_alongside_version_json_defers_to_clap() {
    let argv = argv_tail(&["--version", "--json", "--help"]);
    let mut out = Vec::new();
    let handled = maybe_print_version_json_from("my-tool", "1.2.3", &argv, &mut out);
    assert!(!handled, "help is clap's display flag, not a foreign arg");
    assert!(out.is_empty());
}

/// End-to-end meter for genesis-7k6: `--version --json` plus an unknown arg
/// must exit non-zero with an error envelope naming the offending argument
/// — not silently succeed. Run through the probe bin because the mixed-line
/// path terminates the process (like clap's own `parse()`).
#[test]
fn version_json_with_unknown_arg_exits_nonzero_with_error_envelope() {
    for line in [
        vec!["--version", "--json", "--definitely-not-a-flag"],
        vec!["--definitely-not-a-flag", "--version", "--json"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_version-json-probe"))
            .args(&line)
            .output()
            .expect("spawn version-json-probe");
        assert_eq!(
            output.status.code(),
            Some(2),
            "unknown arg alongside --version --json must exit non-zero (clap's usage code); line: {line:?}"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!("usage path must print exactly one JSON envelope: {e}\nstdout: {stdout}")
        });
        assert_eq!(parsed["ok"], serde_json::json!(false), "line: {line:?}");
        assert_eq!(
            parsed["envelope_kind"],
            serde_json::json!("error"),
            "line: {line:?}"
        );
        let message = parsed["data"]["message"].as_str().unwrap_or_default();
        assert!(
            message.starts_with("error: unexpected argument"),
            "message must mirror clap's usage error, got: {message}"
        );
        assert!(
            message.contains("--definitely-not-a-flag"),
            "error must name the offending argument, got: {message}"
        );
        let remediation = parsed["data"]["remediation"][0]["command"]
            .as_str()
            .unwrap_or_default();
        assert_eq!(remediation, "probe-tool --help", "line: {line:?}");
    }
}

/// The bare `--version --json` fast path is unchanged end to end: version
/// envelope, exit 0.
#[test]
fn version_json_alone_exits_zero_with_version_envelope() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_version-json-probe"))
        .args(["--version", "--json"])
        .output()
        .expect("spawn version-json-probe");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("version path prints one JSON envelope");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert_eq!(parsed["envelope_kind"], serde_json::json!("version"));
    assert_eq!(parsed["data"]["name"], serde_json::json!("probe-tool"));
    assert_eq!(parsed["data"]["version"], serde_json::json!("1.2.3"));
}

/// `tool --version --bogus` (no `--json`) is NOT this ticket: clap's own
/// `--version` action handles the whole line and exits 0 — clap's standard
/// behavior, unchanged by the pre-parse.
#[test]
fn plain_version_with_unknown_arg_is_claps_business() {
    let err = Cli::try_parse_from(argv_tail(&["--version", "--definitely-not-a-flag"]))
        .expect_err("clap must own this line");
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    assert_eq!(err.exit_code(), 0, "clap's version action exits 0");
}
