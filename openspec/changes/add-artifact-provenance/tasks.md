# Tasks: add-artifact-provenance

Ordered per repo TDD convention: each item is a red→green→(refactor) cycle.
Validation after every task: `just test` (or `cargo test --all-features`).

## 1. Managed-block provenance footer

- [x] 1.1 RED: test — footer-less injection is byte-identical to current
      output (lock existing behavior first)
- [x] 1.2 GREEN: add opt-in footer builder (`BlockDef::with_provenance` or
      injector builder) writing `BEGIN:name (version, sha8, source)` footer
      into `src/managed_block.rs`
- [x] 1.3 Test — content hash covers only inter-marker content; re-inject
      updates the hash (spec `managed-block` scenarios)
- [x] 1.4 Refactor: extract hash helper (reuse feedback `repro_hash` style);
      clippy + rustfmt clean

## 2. AIX artifact provenance

- [x] 2.1 RED: test — `generate_llms_txt` output unchanged (determinism guard)
- [x] 2.2 GREEN: add footer line + `*_timestamped` variants to `generate_llms_txt`,
      `generate_llm_txt`, and bounded variants (`src/aix.rs`)
- [x] 2.3 Test — bounded degradation still deterministic; footer hash reflects
      degraded content (spec `aix` delta scenarios)

## 3. ManagedBlockDrift lint check

- [x] 3.1 RED: test — drifted block yields warning `LintResult` with file,
      block name, and caller fix command; current block yields none
- [x] 3.2 GREEN: implement `ManagedBlockDrift` implementing `LintCheck`
      (hash fast-path, full-text fallback) in `src/suite_linter.rs`
- [x] 3.3 Test — hand-edited block (markers intact) is a finding
- [x] 3.4 Register in `LinterRegistry`; unit-test registry wiring

## 4. Receipt terminal-outcome eval check

- [ ] 4.1 RED: test — final envelope with receipt passes; missing receipt is
      `tool_fault`; `success` receipt over `ok: false` is a tool fault citing
      the contradiction (spec `evals` delta scenarios)
- [ ] 4.2 GREEN: implement `receipt_records_terminal_outcome` check fn in
      `src/evals.rs` alongside `ok_envelope` / `doc_drift_blindness`
- [ ] 4.3 Test — check composes into an existing Scenario without breaking
      current checks

## 5. aix-gap feedback kind

- [ ] 5.1 RED: test — `--kind aix-gap` passes validation; `aix_gap`/`aixgap`
      yield a typo suggestion (spec `feedback` delta scenarios)
- [ ] 5.2 GREEN: extend `VALID_KINDS` in `src/feedback.rs`; verify redaction
      and ContextBundle capture route unchanged
- [ ] 5.3 Test — end-to-end: aix-gap capture → `from_feedback_context` →
      replayable Scenario (extends the existing doc example)

## 6. Docs & validation

- [ ] 6.1 Update module docs (footer format, check names) and `llms.txt`
      module table if it lists module functions
- [ ] 6.2 `openspec validate add-artifact-provenance --strict` passes
- [ ] 6.3 CHANGELOG entry under Unreleased (pending EXCL-002 decision)
- [ ] 6.4 Follow-up beads: per-repo footer/drift-lint adoption tickets (wai,
      dont, pretender, testaruda); CI eval-gate recipe ticket
