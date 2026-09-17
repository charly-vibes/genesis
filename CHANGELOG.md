# Changelog

All notable changes to `genesis-vibes` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Envelope receipt metadata** ([genesis-f2o]): `Envelope` gains optional
  `receipt: Option<ReceiptMeta>` with a `with_receipt()` builder, recording
  the terminal outcome (`TerminalOutcome`: `Success`/`Failure`/`Timeout`/
  `Cancelled`), retry identity (`attempt: u32`, optional
  `idempotency_key`), and user-visible `evidence` of a command run
  (add-aix-eval-loop §1).
  - **Compatibility note for downstream tools:** fully additive. Envelopes
    constructed without `with_receipt()` serialize byte-identically to
    previous output (the `receipt` key is omitted; golden-file tested), and
    `parse_envelope` accepts enriched envelopes with no change to its
    return type. No action required.

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
[genesis-f2o]: https://github.com/charly-vibes/genesis
