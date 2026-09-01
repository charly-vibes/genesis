//! Compilable mirrors of the mdBook code snippets on the onboarding path.
//!
//! Every test here corresponds to a fenced `rust` block in the book. When a
//! snippet in `docs/` changes, update the matching test — and vice versa.
//! This exists so API drift between docs and code fails `cargo test`
//! (genesis-0ut), instead of shipping broken onboarding examples.
//!
//! Mapping:
//! - `step2_output`              → docs/getting-started.md, Step 2
//! - `step3_emit_cli_format`     → docs/getting-started.md, Step 3
//! - `step4_version_json`        → docs/getting-started.md, Step 4
//! - `step4_version_json_from_args` → src/cli.rs, maybe_print_version_json_from
//! - `guide_emit_dispatch`       → docs/how-to/guide.md, "Format-dispatching with emit()"
//! - `guide_error_sink`          → docs/how-to/guide.md, "Error handling with ErrorSink"
//! - `envelope_error_result`     → docs/how-to/envelope.md, "Returning errors"
//! - `envelope_read_envelope`    → docs/how-to/envelope.md, "Reading the envelope" (success)
//! - `envelope_read_error_envelope` → docs/how-to/envelope.md, "Reading the envelope" (error)

use genesis::cli::{maybe_print_version_json, maybe_print_version_json_from};
use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry};
use genesis::guide::{CliFormat, CliVerbosity, ErrorSink, Output, OutputFormat, Verbosity};

/// Buffered stdout/stderr pair so tests don't write to the real terminal.
struct Streams {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Streams {
    fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
    fn as_strings(&self) -> (String, String) {
        (
            String::from_utf8_lossy(&self.stdout).into_owned(),
            String::from_utf8_lossy(&self.stderr).into_owned(),
        )
    }
}

/// docs/getting-started.md, Step 2 — structured output with a next step.
#[test]
fn step2_output() {
    let output = Output::success("Project initialized")
        .with_next_step("Run `my-tool doctor` to verify setup");

    let verbosity = Verbosity::Normal;
    let mut streams = Streams::new();

    output
        .print(verbosity, &mut streams.stdout, &mut streams.stderr)
        .expect("print succeeds");

    let (stdout, stderr) = streams.as_strings();
    assert!(stdout.contains("Project initialized"), "stdout: {stdout:?}");
    assert!(
        stderr.contains("Run `my-tool doctor` to verify setup"),
        "stderr: {stderr:?}"
    );
}

/// docs/getting-started.md, Step 3 — `--json` output with format auto-detection.
#[test]
fn step3_emit_cli_format() {
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "my-tool")]
    struct Cli {
        #[command(flatten)]
        verbose: CliVerbosity,
        #[command(flatten)]
        format: CliFormat,
    }

    let cli = Cli::parse_from(["my-tool"]);
    let format: OutputFormat = cli.format.format();
    let verbosity: Verbosity = cli.verbose.verbosity();

    let output = Output::success(vec!["item1", "item2"]);

    // Human branch
    let mut streams = Streams::new();
    output
        .emit(
            "0.0.0-test", // your tool's version, not genesis's
            format,
            verbosity,
            &mut streams.stdout,
            &mut streams.stderr,
        )
        .expect("human emit succeeds");
    let (stdout, _) = streams.as_strings();
    assert!(stdout.contains("item1"), "stdout: {stdout:?}");

    // JSON branch (piped/CI context)
    let mut streams = Streams::new();
    output
        .emit(
            "0.0.0-test",
            OutputFormat::Json,
            verbosity,
            &mut streams.stdout,
            &mut streams.stderr,
        )
        .expect("json emit succeeds");
    let (stdout, _) = streams.as_strings();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("JSON emit must produce valid JSON");
    assert_eq!(parsed["ok"], serde_json::json!(true));
}

/// docs/getting-started.md, Step 4 — pre-parse `--version --json`.
#[test]
fn step4_version_json() {
    // No `--version` in the test runner's args → returns false and continues.
    assert!(!maybe_print_version_json("my-tool", "0.1.0"));
}

/// docs/getting-started.md, Step 4 — the explicit-args variant covers all
/// branches in-process (genesis-r0p): args come in, output goes to a writer.
#[test]
fn step4_version_json_from_args() {
    // `--version --json` → prints the version envelope to the writer, true
    let mut out: Vec<u8> = Vec::new();
    let printed = maybe_print_version_json_from(
        "my-tool",
        "1.2.3",
        &[
            "my-tool".to_string(),
            "--version".to_string(),
            "--json".to_string(),
        ],
        &mut out,
    );
    assert!(printed, "--version --json must report handled");

    let parsed: serde_json::Value =
        serde_json::from_slice(&out).expect("version envelope is valid JSON");
    assert_eq!(parsed["envelope_kind"], serde_json::json!("version"));
    assert_eq!(parsed["data"]["version"], serde_json::json!("1.2.3"));

    // `-V -j` short forms also trigger
    let mut out: Vec<u8> = Vec::new();
    let printed = maybe_print_version_json_from(
        "my-tool",
        "1.2.3",
        &["my-tool".to_string(), "-V".to_string(), "-j".to_string()],
        &mut out,
    );
    assert!(printed);
    assert!(!out.is_empty());

    // Plain `--version` (no --json) → left for clap, false, nothing printed
    let mut out: Vec<u8> = Vec::new();
    let printed = maybe_print_version_json_from(
        "my-tool",
        "1.2.3",
        &["my-tool".to_string(), "--version".to_string()],
        &mut out,
    );
    assert!(!printed, "plain --version is clap's job");
    assert!(out.is_empty());

    // No version flag at all → false
    let mut out: Vec<u8> = Vec::new();
    let printed =
        maybe_print_version_json_from("my-tool", "1.2.3", &["my-tool".to_string()], &mut out);
    assert!(!printed);
    assert!(out.is_empty());
}

/// docs/how-to/guide.md — "Format-dispatching with emit()".
#[test]
fn guide_emit_dispatch() {
    fn list_items(
        cli_version: &str,
        format: OutputFormat,
        verbosity: Verbosity,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = Output::success(vec!["item1", "item2", "item3"]);

        let mut streams = Streams::new();
        output.emit(
            cli_version,
            format,
            verbosity,
            &mut streams.stdout,
            &mut streams.stderr,
        )?;
        Ok(())
    }

    list_items("0.0.0-test", OutputFormat::Human, Verbosity::Normal).expect("human emit");
    list_items("0.0.0-test", OutputFormat::Json, Verbosity::Normal).expect("json emit");
}

/// docs/how-to/guide.md — "Error handling with ErrorSink".
#[test]
fn guide_error_sink() {
    let sink = ErrorSink::new("my-tool");
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "config.toml not found");

    let mut streams = Streams::new();
    sink.handle(&err, &mut streams.stderr);

    let suggestion = genesis::suggestions::Suggestion::Fix {
        description: "create a default config".to_string(),
        command: Some("my-tool init".to_string()),
    };
    sink.handle_with_footer(&err, &suggestion, &mut streams.stderr);

    let (_, stderr) = streams.as_strings();
    assert!(
        stderr.contains("config.toml not found"),
        "stderr: {stderr:?}"
    );
    assert!(stderr.contains("my-tool init"), "stderr: {stderr:?}");
}

/// docs/how-to/envelope.md — "Returning errors" (Invariant 3.2.5).
#[test]
fn envelope_error_result() {
    // Good — remediation is non-empty
    let err = ErrorResult::new(
        "E_CONFIG_NOT_FOUND",
        "config file not found",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "my-tool init".to_string(),
            description: "Create a default config".to_string(),
        }],
    )
    .expect("non-empty remediation passes");

    assert_eq!(err.remediation[0].command, "my-tool init");

    // Bad — empty remediation is rejected
    let result = ErrorResult::new(
        "E_BROKE",
        "something broke",
        None,
        None,
        None,
        vec![],
        vec![],
    );
    assert!(result.is_err(), "empty remediation must fail");
}

/// docs/how-to/envelope.md — "Reading the envelope" (check `ok` first).
#[test]
fn envelope_read_envelope() {
    let env: Envelope<&str> = Envelope::success(
        "0.0.0-test",
        EnvelopeKind::Ok,
        "operation completed",
        vec![],
        vec![],
    );

    assert!(env.ok, "success envelope has ok=true");
    assert_eq!(env.cli_version, "0.0.0-test");

    let serialized = serde_json::to_string(&env).expect("envelope serializes");
    let parsed: serde_json::Value =
        serde_json::from_str(&serialized).expect("envelope round-trips");
    assert_eq!(parsed["envelope_kind"], serde_json::json!("ok"));
}

/// docs/how-to/envelope.md — "Reading the envelope", error branch:
/// an error envelope carries an ErrorResult as its data.
#[test]
fn envelope_read_error_envelope() {
    let err = ErrorResult::new(
        "E_CONFIG_NOT_FOUND",
        "config file not found",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "my-tool init".to_string(),
            description: "Create a default config".to_string(),
        }],
    )
    .expect("non-empty remediation");

    let env: Envelope<ErrorResult> = Envelope::error("0.0.0-test", err, vec![]);
    assert!(!env.ok);
    assert_eq!(env.data.code, "E_CONFIG_NOT_FOUND");

    // Mirror the doc snippet's rendering
    assert!(
        format!("Error [{}]: {}", env.data.code, env.data.message)
            .contains("config file not found")
    );
    for entry in &env.data.remediation {
        assert_eq!(
            format!("  → {} — {}", entry.command, entry.description),
            "  → my-tool init — Create a default config"
        );
    }
}
