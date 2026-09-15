# Changelog

All notable changes to `genesis-vibes` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- **Exit-code contract refinement** ([genesis-u40]): `Guide::run` and
  `Guide::run_formatted` now return `2` when output emission fails with an
  I/O error (internal failure). Previously every failure exited `1`.
  - `0` — success; `1` — user-facing error (unchanged, backward
    compatible); `2` — internal failure; panics unwind with Rust's
    default behavior (no panic hook is installed, stack traces are never
    swallowed).
  - **Compatibility note for downstream tools:** exit `1` remains
    reserved for user-facing errors, so existing `exit == 1`
    user-error assertions keep working unchanged. If your eval harness
    or shell script asserts on exit `1` for *internal* I/O failures
    (previously indistinguishable), update those assertions to `2`.

[genesis-u40]: https://github.com/charly-vibes/genesis
