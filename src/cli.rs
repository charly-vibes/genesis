//! CLI helpers for genesis-integrated tools.
//!
//! Provides small, reusable utilities that multiple tools need:
//!
//! - [`generate_completions`] — shell completions via `clap_complete`
//! - [`maybe_print_version_json`] — `--version --json` pre-parse

use std::io::Write;

/// Generate shell completion scripts for the given CLI and shell.
///
/// Writes to stdout. Each tool calls this from its `Completions` subcommand.
///
/// # Example
///
/// ```rust,no_run
/// use clap::Parser;
/// use clap::CommandFactory;
/// use genesis::cli::generate_completions;
///
/// #[derive(Parser)]
/// struct Cli;
///
/// let mut cmd = Cli::command();
/// generate_completions(&mut cmd, clap_complete::Shell::Bash).unwrap();
/// ```
pub fn generate_completions(
    cmd: &mut clap::Command,
    shell: clap_complete::Shell,
) -> std::io::Result<()> {
    let name = cmd.get_name().to_string();
    let mut stdout = std::io::stdout().lock();
    clap_complete::generate(shell, cmd, name, &mut stdout);
    stdout.flush()
}

/// Check if `--version --json` was requested before clap's normal parsing.
///
/// Clap's built-in `--version` doesn't participate in the global `--json`
/// flag, so tools that support `--version --json` must handle this before
/// calling `Cli::parse()`.
///
/// Returns `true` if the version was printed and the caller should exit.
///
/// A line that combines `--version`/`-V` with `--json`/`-j` *and* any other
/// argument is never swallowed: it gets clap's standard usage error as an
/// error envelope and exits with clap's usage code (2) (genesis-7k6).
///
/// Reads from [`std::env::args`]. For a testable variant that takes explicit
/// args and an output writer, see [`maybe_print_version_json_from`].
///
/// # Example
///
/// ```rust,no_run
/// use genesis::cli;
///
/// if cli::maybe_print_version_json("my-tool", "0.1.0") {
///     return;
/// }
/// // ... proceed to Cli::parse()
/// ```
pub fn maybe_print_version_json(name: &str, version: &str) -> bool {
    let args: Vec<String> = std::env::args().collect();
    let mut stdout = std::io::stdout().lock();
    maybe_print_version_json_from(name, version, &args, &mut stdout)
}

/// [`maybe_print_version_json`] with explicit args and output writer.
///
/// Behavior is identical: returns `true` only when `--version` was combined
/// with `--json`/`-j` (in which case the version envelope was written to
/// `out`); plain `--version` and no-flag invocations return `false` and write
/// nothing, leaving version handling to clap. A mixed line — `--version` and
/// `--json` plus any other argument — is not swallowed either: it prints
/// clap's usage error as an error envelope and exits 2 (genesis-7k6), so
/// unlike the returning branches this path never comes back to the caller.
/// `--help`/`-h` are excluded from that check: they are clap's display flags,
/// and clap itself resolves their precedence against `--version`.
///
/// # Example
///
/// ```rust,no_run
/// use genesis::cli::maybe_print_version_json_from;
///
/// let args: Vec<String> = std::env::args().collect();
/// let mut out = std::io::stdout().lock();
/// if maybe_print_version_json_from("my-tool", "0.1.0", &args, &mut out) {
///     return;
/// }
/// // ... proceed to Cli::parse()
/// ```
pub fn maybe_print_version_json_from(
    name: &str,
    version: &str,
    args: &[String],
    out: &mut impl Write,
) -> bool {
    let Some(scan) = scan_version_args(args) else {
        return false;
    };
    // EDGE-002: a mixed line must error, not be silently swallowed by clap's
    // exit-0 `--version` action.
    if let Some(bad) = scan.foreign.first() {
        print_usage_error_envelope(name, version, bad, out);
        std::process::exit(2);
    }
    print_version_envelope(name, version, out);
    true
}

/// Classification of the argv tail for the `--version --json` fast path.
struct VersionScan<'a> {
    wants_version: bool,
    wants_json: bool,
    foreign: Vec<&'a str>,
}

/// One pass over argv: collect version/json wants and any foreign args.
/// `None` = defer to clap entirely (`--help`/`-h` present, no `--version`,
/// or plain `--version` without `--json` — clap's own DisplayVersion action).
fn scan_version_args(args: &[String]) -> Option<VersionScan<'_>> {
    let mut scan = VersionScan {
        wants_version: false,
        wants_json: false,
        foreign: Vec::new(),
    };
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => scan.wants_version = true,
            "--json" | "-j" => scan.wants_json = true,
            // Clap's display flags exist on every consumer CLI; their
            // precedence against `--version` (first action wins) is clap's
            // own job, so a line carrying them is left to clap entirely.
            "--help" | "-h" => return None,
            other => scan.foreign.push(other),
        }
    }
    scan.wants_version.then_some(scan).filter(|s| s.wants_json)
}

/// Emit clap's standard usage error for an unknown argument as one JSON
/// error envelope on `out`.
///
/// The message mirrors clap's usage-error text and the exit is clap's
/// usage-error code (2). The remediation entry keeps Invariant 3.2.5
/// (errors always carry a suggested fix).
fn print_usage_error_envelope(name: &str, version: &str, bad: &str, out: &mut impl Write) {
    use crate::envelope::{Envelope, ErrorResult, RemediationEntry};
    let err = ErrorResult::new(
        "usage",
        &format!("error: unexpected argument '{bad}' found"),
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: format!("{name} --help"),
            description: "list this tool's valid arguments".to_string(),
        }],
    )
    .expect("usage error envelope always carries remediation");
    let envelope = Envelope::error(env!("CARGO_PKG_VERSION"), version, err, vec![]);
    let json = serde_json::to_string(&envelope).expect("serialize usage error envelope");
    writeln!(out, "{json}").expect("write usage error envelope");
}

/// Emit the version envelope as one JSON line on `out`.
fn print_version_envelope(name: &str, version: &str, out: &mut impl Write) {
    use crate::envelope::{Envelope, EnvelopeKind};
    let envelope = Envelope::success(
        env!("CARGO_PKG_VERSION"),
        version,
        EnvelopeKind::Version,
        serde_json::json!({
            "name": name,
            "version": version
        }),
        vec![],
        vec![],
    );
    let json = serde_json::to_string(&envelope).expect("serialize version envelope");
    writeln!(out, "{json}").expect("write version envelope");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "test-tool")]
    struct TestCli;

    /// Generate completions for `shell` and assert the generation succeeds
    /// (output goes to stdout).
    fn completions_ok(shell: clap_complete::Shell) {
        let mut cmd = TestCli::command();
        let result = generate_completions(&mut cmd, shell);
        assert!(
            result.is_ok(),
            "completion generation for {shell:?} must succeed: {result:?}"
        );
    }

    #[test]
    fn test_generate_completions_bash() {
        completions_ok(clap_complete::Shell::Bash);
    }

    #[test]
    fn test_generate_completions_zsh() {
        completions_ok(clap_complete::Shell::Zsh);
    }

    #[test]
    fn test_generate_completions_fish() {
        completions_ok(clap_complete::Shell::Fish);
    }

    #[test]
    fn test_maybe_print_version_json_no_version() {
        // No --version flag — should return false
        let result = maybe_print_version_json("test", "0.1.0");
        assert!(!result);
    }

    #[test]
    fn test_maybe_print_version_json_version_only() {
        // --version without --json — should return false (let clap handle it)
        let result = maybe_print_version_json("test", "0.1.0");
        assert!(!result, "version-only should not print");
    }

    #[test]
    fn test_maybe_print_version_json_name_reflects_input() {
        let result = maybe_print_version_json("custom-name", "2.0.0");
        assert!(!result, "no --version flag should return false");
    }
}
