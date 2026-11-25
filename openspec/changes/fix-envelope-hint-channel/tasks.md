# Tasks: fix-envelope-hint-channel

Tracked as `genesis-jsu` in beads (base_commit `3bffa47`).

Meter: `just ci` (fmt, clippy, test, docs, build-release, aix-check) and
`ah check` (spec-test correspondence).

## 1. Reader (genesis::evals)

- [ ] 1.1 RED: `tests/evals_hint_channel.rs` — `parse_envelope` resolves the
      `data.remediation` command; resolves a top-level `hints` command;
      resolves both, error channel first; tolerates a bare-string hint entry;
      an envelope with neither channel yields zero commands and no error.
- [ ] 1.2 GREEN: extend `parse_envelope` (`src/evals/outcome.rs`) to read
      `data.remediation[].command` and `hints[].command`, accepting object or
      string entries; union order = error channel first.
- [ ] 1.3 `EnvelopeOutcome::Error::hint_commands` doc comment updated to
      state that the list is the union of both channels.

## 2. Fixtures (genesis::evals)

- [ ] 2.1 RED: `tests/evals_error_fixture.rs` — a fixture built with
      `Envelope::error` + `ErrorResult` (one remediation entry) resolves to a
      non-empty hint-command list; a fixture built with `Envelope::success`
      parses as `Ok`.
- [ ] 2.2 GREEN: add a fixture helper that builds envelopes through the
      genesis constructors; replace the hand-written JSON literals in
      `src/evals/tests_e2e.rs:11-18` (error path) **and** `:20-22`
      (`ok_envelope_stdout`, success path) with it — the spec requires every
      scenario fixture to come from the constructors.
- [ ] 2.3 Re-run the hint-following scenarios (`hint_followed`,
      `error_envelope_with_remediation_hint`, `agent_followed_hint`) against
      the genesis-built fixture; the suite SHALL pass.
- [ ] 2.4 Re-run the genesis eval suite and report every classification flip
      (a scenario that previously reported `ERR_ENVELOPE_HINT_BLINDNESS` and
      now passes is a correction, not a regression — each flip is reviewed,
      not absorbed).

## 3. Spec

- [ ] 3.1 Spec delta `specs/envelope/spec.md` — error remediation channel,
      success-path hint channel, reader tolerance for the deprecated
      error-path channel (done in this change).
- [ ] 3.2 Spec delta `specs/evals/spec.md` — dual-channel resolution,
      genesis-built fixtures, hint checks validated against the real shape
      (done in this change).
- [ ] 3.3 On archive: deploy `openspec/specs/envelope/spec.md` and
      `openspec/specs/evals/spec.md`.

## 4. Contract + gates

- [ ] 4.1 `.espectacular/evals/` contract TOMLs for the new scenarios
      (dual-channel resolution, string-valued entries, genesis-built fixture),
      mapped to test flags.
- [ ] 4.2 `ah check` green.
- [ ] 4.3 `just ci` green.

## 5. Cross-repo follow-up (filed at approval, not implemented here)

- [ ] 5.1 evallerina: replace the private dual-channel reader
      (`evallerina/src/envelope.rs:60-84`) with `genesis::evals::parse_envelope`
      + the union helper; **re-run the recorded scenario corpus**
      (`evallerina/src/recorded.rs`, `scenarios/*.json`) and reconcile every
      classification flip against the pre-change report.
- [ ] 5.2 CHANGELOG entry naming the channel contract (genesis side).
- [ ] 5.3 `versions.ddl.toml` **patch** bump entry (reader behavior change,
      no public signature change).
