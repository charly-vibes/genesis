# Change: Add `update_check` module (crates.io update notifications)

## Why

Suite tools are updated frequently with no clear release cadence (wai tags:
v2026.7.28, v2026.7.29, v2026.7.31, v2026.8.5, v2026.9.28), so users can run
a stale binary for weeks without noticing. Pull-only checks (doctor/version)
fire too rarely to catch staleness, because check rate is decoupled from usage
rate. Reviewed via Rule-of-5 (converged); tracked as genesis-2ex.

A cached-passive update check is needed by **three** suite tools (wai,
pretender, testaruda) — it passes the genesis boundary rule. Each binary
notifies about ITS OWN crate; genesis only supplies the mechanism.

## What Changes

### `genesis::update_check` — feature-gated module

```rust
use genesis::update_check::{check, notice};

// In a dependent binary's CLI, after command output:
if let Some(info) = check(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")) {
    eprintln!("{}", notice(&info));
    // "wai 2026.10.12 available — you have 2026.9.28 (cargo install wai)"
}
```

- **Cached-passive**: at most one fetch per cache TTL (default 7 days);
  cache at `<cache>/genesis/update-check/<crate>.json` (XDG_CACHE_HOME,
  fallback `$HOME/.cache`), atomic write (temp + rename).
- **Fail-silent**: any error (network, timeout, corrupt cache, unwritable
  dir) degrades to `None`. Never panics, never surfaces errors, 2s timeout.
- **CI-aware**: `CI=true` or `GENESIS_NO_UPDATE_CHECK=<non-empty>` skips
  before any IO.
- **Polite**: descriptive User-Agent; crates.io 403/429 doubles the TTL.
- **Scheme-agnostic**: comparison is `current != latest`; yanked and semver
  pre-release versions are filtered out; calendar versions need no
  special-casing.
- **Feature-gated**: `update-check` feature pulls `ureq` + `semver`;
  dependents without the feature pull no HTTP stack.

## Impact

- **Affected specs**: new `update-check` spec
- **Affected code**: `src/update_check.rs` (new), `src/lib.rs` (cfg-gated
  module), `Cargo.toml` (optional deps + feature), `justfile` (test runs
  `--all-features` so spec contracts are non-vacuous)
- **Downstream wiring** (separate tickets, one per repo): wai, pretender,
  testaruda wire `check()` into their CLI binaries; free pull-style reuse in
  their doctor/version subcommands.