# Changelog

All notable changes to `genesis-vibes` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed

- **git_hooks: lefthook managed blocks now nest inside the stage's
  `commands:` mapping** — `ensure_wired` previously inserted the block
  directly after the stage key, producing a stage-level command key that
  lefthook silently ignores at runtime (only `lefthook validate` rejects
  it), so downstream wiring like espectacular's `ah check` never ran
  (genesis-r99, charly-vibes/espectacular#31). Caller content is now the
  entries inside `commands:` (inserted verbatim, 4-space indent at the
  `commands:` child level); `ensure_wired` emits `  commands:` after the
  stage anchor when the stage lacks one, and appended missing stages get
  `commands:` too. **Breaking for callers that passed a stage-level
  mapping** — re-indent content one level deeper.

## [0.11.1] — 2026-10-01

### Fixed

- **feedback: git branch gathered on unborn HEAD** — `get_git_branch` now
  uses `git branch --show-current` instead of `rev-parse --abbrev-ref
  HEAD`, so a freshly `git init`-ed repo (no commits yet) still reports
  its branch name in the feedback context bundle; empty output (detached
  HEAD) degrades to omitted. Non-Rust-workspace graceful degradation of
  `gather_context` is now pinned by regression tests (downstream:
  espectacular GH#28 item 3 — the hard `Cargo.toml` read there lives in
  ah's own main.rs, not genesis).
- **git_hooks: comment lines no longer trigger the lefthook
  `UnanchorableLefthookConfig` refusal** — `find_anchor` now skips comment
  lines entirely, so a config whose stage name appears in comments before a
  clean column-0 `pre-commit:`/`pre-push:` key anchors normally (downstream:
  espectacular GH#28 item 1, tracked by espectacular-n4v). Comment-only
  mentions are treated as an absent stage (section is appended).

## [0.11.0] — 2026-10-01

### Added

- **Artifact provenance & staleness loop** ([add-artifact-provenance]):
  managed-block injection can append an opt-in provenance footer
  (`generator`/`version`/`source`/`sha`) whose hash covers only the block
  content, so version bumps never move the hash. New `ManagedBlockDrift`
  lint (`genesis.managed_block_drift`) detects drifted blocks via the
  footer hash (fast path) or full-text comparison (footer-less blocks);
  new `receipt_records_terminal_outcome` eval check faults missing
  receipts and `success` receipts over `ok:false` envelopes; `feedback
  --kind aix-gap` files AIX-artifact failures through the standard
  pipeline and converts to replayable regression scenarios.

## [0.10.1] — 2026-10-01

### Fixed

- **update-check: `cache_path` honors `XDG_CACHE_HOME` as the cache root**
  ([genesis-39r]): the cache path was `\$XDG_CACHE_HOME/.cache/genesis/...`;
  per the XDG Base Directory spec — and the codebase precedent in
  `feedback/scratch.rs` — it is now `\$XDG_CACHE_HOME/genesis/...` when the
  variable is set, falling back to `\$HOME/.cache` otherwise.

- **update-check/test files: locals named `raw` referenced as `&raw`
  renamed** ([genesis-kn6]): a local named `raw` borrowed as `&raw`
  collides with the `&raw const/mut` expression in tree-sitter-rust
  0.23.3 (pretender's pinned grammar), producing spurious "Parse errors
  detected" warnings. Workaround genesis-side; the real fix is upstream
  (pretender-mlw: bump tree-sitter-rust 0.23 → 0.24, which parses it
  clean).
- **docs: install pins 0.9 → 0.10 in README/getting-started** missed in
  the v0.10.0 release commit (caught by the doc_sync drift guard).

### Refactored

- **git-hooks: `ensure_command_wired` decomposed into single-
  responsibility helpers** ([genesis-8og]): `marker_outcome`,
  `entry_at`/`wrapper_at`, `infer_entry_indent`, `insert_after_line`,
  `wired_at_anchor`, `insert_wrapper`, `append_missing_stage`.
  function_lines 146 → 33, abc 77.32 → 16.16, cyclomatic 9 → 2.
  Behavior-preserving — the 12 wiring tests are byte-identical.

## [0.10.0] — 2026-10-01

### Added

- **git-hooks: `lefthook::ensure_command_wired()`** ([genesis-au8]):
  command-level wiring — inserts a marker-guarded command entry *inside*
  an existing stage `commands:` mapping at the mapping's own entry indent
  (per-level YAML indent is config-dependent), and injects a full
  `commands:` wrapper when the stage is missing, empty, or childless.
  Honest refusals, file unmodified: missing config (never created, D4),
  quoted stage key, `commands:` at an indent other than the children's,
  and exactly-one-marker present (new `UnbalancedLefthookMarkers` error).
  Donor: specodelic `src/hooks.rs` (specodelic-x56) — consumers stop
  re-implementing inside-mapping insertion.

### Fixed

- **git-hooks: `ensure_wired` no longer glues the END marker onto the
  next existing line** ([genesis-au8]): a block injected after the stage
  key always ends on its own line now. With comment-prefixed markers the
  glued line turned the following YAML key into a comment — silently
  deleting it (found wiring specodelic-gates).

## [0.9.0] — 2026-09-30

### Fixed

- **update-check: version selection no longer compares `created_at`
  timestamps** ([genesis-4mq]): the first non-yanked stable entry of the
  crates.io response is selected directly. Timestamp-string comparison
  resolved ties to the OLDEST version (crates.io is newest-first).
- **update-check: crate names are validated before use** ([genesis-4mq]):
  names must be non-empty ASCII alphanumeric plus `-`/`_`; anything else
  (e.g. `../` traversal) returns `None` before any IO. Applies to
  `check`, `check_with`, and `cache_path`.
- **update-check: failed fetches preserve a known-good cached `latest`**
  ([genesis-4mq]): backoff entries (transport errors, 404, rate limits)
  keep the previously known `latest`/`published_at` instead of downgrading
  cached knowledge to "no update".
- **update-check: a future `checked_at` (clock skew) is treated as stale**
  ([genesis-4mq]), forcing a refetch instead of being trusted as maximally
  fresh.

### Changed

- **update-check: total request timeout raised from 2s to 5s**
  ([genesis-4mq]) — connect stays at 2s (`CONNECT_TIMEOUT`/`TOTAL_TIMEOUT`
  are now public constants), so responses over slow links still succeed.

### Added

- **update-check: `GENESIS_UPDATE_CHECK_DEBUG=1` emits one stderr line per
  skip/fail reason** ([genesis-4mq]) — de-risks silent 404s when wiring
  dependents (pass YOUR crate name; wai's is `wai-cli`, not `wai`).

[genesis-au8]: https://github.com/charly-vibes/genesis
[genesis-kn6]: https://github.com/charly-vibes/genesis
[genesis-8og]: https://github.com/charly-vibes/genesis
[genesis-4mq]: https://github.com/charly-vibes/genesis
[genesis-39r]: https://github.com/charly-vibes/genesis

## [0.8.3] — 2026-09-29

### Fixed

- **git_hooks: `resolve_hooks_dir()` now resolves `core.hooksPath` across
  all config scopes** ([genesis-c64]): local → global → system, matching
  git's own precedence. Previously only the repo-local value was honored,
  so with a global `core.hooksPath` (e.g. a lefthook shim) hooks installed
  by downstream tools were silently never invoked by git.

### Added

- **git_hooks: `effective_hooks_dir()` + `HooksDirScope`/`EffectiveHooksDir`**
  ([genesis-c64]): resolves across all scopes and reports which scope the
  effective value came from, so doctor-style checks can warn when a hook
  was installed at a different scope than the one git will use.
  Empty-string `core.hooksPath` (git disables hooks) is surfaced as
  `HooksDirScope::Disabled`; `resolve_hooks_dir()` keeps its old fallback
  (default `.git/hooks`) for that case, so existing callers are unaffected.

[genesis-c64]: https://github.com/charly-vibes/genesis/issues/12

## [0.8.2] — 2026-09-29

### Fixed

- **envelope: `ok` now agrees with `envelope_kind`** ([genesis-r13]):
  `Output::to_envelope` routes failed outputs through `Envelope::success`
  with `kind = Error`; the envelope's `ok` field was hardcoded `true`, so
  error JSON read `{"ok":true,"envelope_kind":"error"}`. `ok` is now
  derived from the kind (`false` iff `Error`). Envelopes built with
  `Ok`/`Version`/other kinds are byte-identical.
- **suggestions: no more `→ Run: run: …` doubling** ([genesis-r13]):
  `Suggestion::fix` strips a leading `run: `/`Run: ` from the authored
  hint — the footer already renders `→ Run: {cmd}`, so the prefix used
  to appear twice on stderr and inside the JSON hint.
- **suggestions: trailing-command hints split at the last ` run: `
  ** ([genesis-9us]): hints authored `every reference resolves — run:
  specodelic lint` now yield a runnable `command` (`specodelic lint`)
  and guidance as the `description` (`every reference resolves`); the
  whole sentence no longer ships as the command. Leading-prefix hints
  win over the trailing split.
- **envelope hints: `command` is now the bare runnable command**
  ([genesis-r13]): `Output::to_envelope` used `Suggestion::footer()` for
  `HintEntry.command`, leaking the human `→ Run: ` prefix into JSON.
  New accessors `Suggestion::command()` (bare command) and
  `Suggestion::guidance()` (description without the footer line) back
  the JSON hint; human stderr rendering is unchanged.
- **feedback tests: scratch store isolation** ([genesis-kpv]):
  `test_tool` now appends a per-process counter — the scratch store is
  keyed by tool name, so two calls sharing a label raced on the same
  scratch dir under parallel test execution (one observed flake).

### Downstream compatibility notes

- `HintEntry.command` values change for every tool that passes hints
  through `Output::with_next_step` — consumers parsing the JSON hint's
  command get a runnable command now (previously the human footer line).
- `Suggestion::Fix` command/description contents change per the two
  split rules above; machine consumers of `suggestion` fields in
  serialized envelopes (serde `Suggestion`) see the same shapes with
  cleaner payloads.

## [0.8.1] — 2026-09-28

### Changed

- **feedback: full stdin read + smarter title derivation** ([genesis-gle],
  [genesis-og6]): the stdin path now reads all of piped input
  (`read_to_string`) instead of a single line, so multi-paragraph reports
  are no longer silently truncated. Multi-line input promotes its first
  line to the issue title (remainder becomes the `## Description` body);
  single-line input is unchanged (generic `[{kind}] feedback report` title,
  input verbatim in the Description).
- **feedback: user-supplied title** ([genesis-gle]): `FeedbackArgs` gains
  `title: Option<String>` with a `with_title()` builder; downstream tools
  can expose a `--title` flag. The override wins over every derived title
  (first stdin line, `auto-reported error: …`, generic fallback).
  - **Compatibility note for downstream tools:** `FeedbackArgs::new()`
    keeps its 3-argument signature and single-line stdin behavior is
    byte-identical, so existing callers are unaffected. Constructing
    `FeedbackArgs` via struct literal requires adding the new
    `title: None` field. Downstream tools exposing `feedback` should add
    a `--title` flag and wire it through.

## [0.8.0] — 2026-09-28

### Added

- **Evals guidelines** (`add-evals-guidelines` change, archived): suite-wide
  agent-evals conventions deployed as `openspec/specs/evals-guidelines/spec.md`
  (13 requirements, 32 scenarios).
  - `ErrorTaxonomy::ActionFormatViolation` (`ERR_ACTION_FORMAT_VIOLATION`):
    new taxonomy code for agent action/argument-format faults, with
    round-trip via `from_code`.
  - **Eval report contract v1**: normative report schema
    (`docs/reference/eval-report.md`) with byte-exact golden fixtures
    (`tests/golden/eval_report_{tier2,replay}.json`) and round-trip /
    no-nulls / vocabulary tests.
  - `EvalsGuidelinesAdoption` advisory linter check.
  - mdBook pages: "Running the Eval Tier Ladder in CI"
    (`how-to/evals-ci.md`), "Why weak readers"
    (`explanation/why-weak-readers.md`), and live-cadence guidance in
    `how-to/evals.md`.
  - **Compatibility note for downstream tools:** fully additive — a new
    taxonomy variant (match arms on `ErrorTaxonomy` need the new code),
    otherwise no existing API changed.
- **`git_hooks` module** ([genesis-7l8], [genesis-pzz], [genesis-3a8],
  [genesis-16c], [genesis-qjg]): shared git-hook primitives consolidating
  overlapping mechanics from three donors (pretender install/uninstall with
  ownership markers, wai `core.hooksPath` + owner/framework detection,
  espectacular lefthook injection rebuilt on `managed_block`).
  - **Compatibility note for downstream tools:** fully additive — a new
    public module, no changes to existing APIs. The module contains only
    git-hook mechanics (guard-tested): consuming tools pass their own
    gate commands, markers, and block contents as parameters.
- **AIX module registry complete** ([genesis-qlj]): `evals` and
  `git_hooks` now registered in `examples/gen-aix.rs`, so the packaged
  `llms.txt` / `llm.txt` advertise all 16 modules.

## [0.7.0] — 2026-09-17

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
- **AIX token-cost estimate + bounded generation** ([genesis-35d]):
  `estimate_token_cost(&str) -> TokenCost` (chars/4 heuristic, documented
  ±25% band, heuristic name ships with the estimate) and budget-bounded
  variants `generate_llms_txt_bounded` / `generate_llm_txt_bounded` that
  degrade content deterministically (truncate descriptions → drop raw
  sections → drop tables, headings always survive) instead of overflowing.
  - **Compatibility note for downstream tools:** fully additive. Existing
    `generate_llms_txt` / `generate_llm_txt` signatures and output are
    unchanged (golden-file pinned). No action required.
- **Evals distractors + doc-drift blindness check** ([genesis-lzo]):
  `Scenario::distractor_file(path, content, kind)` declares bait files
  (`DistractorKind::StaleDocs` / `ContradictingHint`) that materialize in the
  replay environment without faulting a run by themselves;
  `doc_drift_blindness(bait)` asserts the agent trusted the tool's envelope
  over stale docs (new taxonomy `ERR_DOC_DRIFT_BLINDNESS`);
  `ScenarioReport` gains optional `model` attribution + JSON serialization
  for per-model matrix runs.
  - **Compatibility note for downstream tools:** additive. `ScenarioResult`
    gains a `distractors` field (only relevant if you construct it by hand —
    use `Scenario::run`); `ScenarioReport` serialization is new, and omits
    `model` when absent. No action required.
- **Feedback → Scenario conversion** ([genesis-m3p]):
  `Scenario::from_feedback_context(bundle, fixtures)` turns captured
  feedback context into a replayable regression scenario embedding the
  recorded failure signature (command, exit code, footer hint);
  `feedback::from_last_error(tool_name, fixtures)` wraps the scratch record
  with typed `ConversionError::NoScratchRecord` on absence. Callers supply
  fixture files explicitly — genesis never re-snapshots the working tree.
  Conversion + replay are pure in-process computation (no LLM, no
  subprocess).
  - **Compatibility note for downstream tools:** fully additive. No action
    required.

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
[genesis-35d]: https://github.com/charly-vibes/genesis
[genesis-lzo]: https://github.com/charly-vibes/genesis
[genesis-m3p]: https://github.com/charly-vibes/genesis
[genesis-7l8]: https://github.com/charly-vibes/genesis
[genesis-pzz]: https://github.com/charly-vibes/genesis
[genesis-3a8]: https://github.com/charly-vibes/genesis
[genesis-16c]: https://github.com/charly-vibes/genesis
[genesis-qjg]: https://github.com/charly-vibes/genesis
[genesis-qlj]: https://github.com/charly-vibes/genesis
[genesis-gle]: https://github.com/charly-vibes/genesis
[genesis-og6]: https://github.com/charly-vibes/genesis
[genesis-r13]: https://github.com/charly-vibes/genesis
[genesis-9us]: https://github.com/charly-vibes/genesis
[genesis-kpv]: https://github.com/charly-vibes/genesis
[add-artifact-provenance]: openspec/changes/add-artifact-provenance
