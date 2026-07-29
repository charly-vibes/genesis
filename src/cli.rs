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

    let has_version = args.iter().any(|a| a == "--version" || a == "-V");

    if !has_version {
        return false;
    }

    let has_json = args.iter().any(|a| a == "--json" || a == "-j");

    if has_json {
        use crate::envelope::{Envelope, EnvelopeKind};
        let envelope = Envelope::success(
            EnvelopeKind::Version,
            serde_json::json!({
                "name": name,
                "version": version
            }),
            vec![],
            vec![],
        );
        println!("{}", serde_json::to_string(&envelope).unwrap());
        true
    } else {
        // Plain --version: let clap handle it
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "test-tool")]
    struct TestCli;

    #[test]
    fn test_generate_completions_bash() {
        let mut cmd = TestCli::command();
        // Just verify it doesn't panic — output goes to stdout
        let result = generate_completions(&mut cmd, clap_complete::Shell::Bash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_completions_zsh() {
        let mut cmd = TestCli::command();
        let result = generate_completions(&mut cmd, clap_complete::Shell::Zsh);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_completions_fish() {
        let mut cmd = TestCli::command();
        let result = generate_completions(&mut cmd, clap_complete::Shell::Fish);
        assert!(result.is_ok());
    }

    #[test]
    fn test_maybe_print_version_json_no_version() {
        // No --version flag — should return false
        let result = maybe_print_version_json("test", "0.1.0");
        assert!(!result);
    }

    #[test]
    fn test_maybe_print_version_json_version_only() {
        // --version without --json — let clap handle it
        // We can't easily test this without env manipulation
    }
}
