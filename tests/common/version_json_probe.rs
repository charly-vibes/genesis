//! Test harness: the documented genesis consumer pattern (getting-started
//! Step 4) as a runnable binary.
//!
//! Integration tests need a real process to assert exit codes and the exact
//! envelope the `--version --json` pre-parse emits, and `CARGO_BIN_EXE_*`
//! only points at bin targets — so this bin exists. It is not part of the
//! published package (`tests/` is excluded from `include`).
//!
//! Beyond `--version`/`--json` handling this main intentionally does nothing
//! else: the probe only exercises the pre-parse, never a clap parse.

fn main() {
    // A real consumer `return`s from main once the envelope is printed; main
    // ends here anyway, so there is nothing further to do.
    let _handled = genesis::cli::maybe_print_version_json("probe-tool", "1.2.3");
}
