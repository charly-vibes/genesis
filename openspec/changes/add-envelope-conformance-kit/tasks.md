# Tasks: add-envelope-conformance-kit

Tracked as `genesis-ffc` in beads (base_commit `3bffa47`).

Meter: `just ci` and `ah check`.

## 1. Module (genesis)

- [ ] 1.1 RED: `tests/conformance_shape.rs` — a genesis-shaped envelope passes
      the shape check; an envelope missing `envelope_kind` (and each other
      member in turn) fails, and the failure names the missing member.
- [ ] 1.2 GREEN: `src/conformance.rs` — the shape check.
- [ ] 1.3 RED: `tests/conformance_channels.rs` — an error envelope with
      `data.remediation` passes the error check and the hint check; an error
      envelope with absent or empty remediation fails the error check; an
      envelope naming no command in any channel fails the hint check; the hint
      check resolves through `genesis::evals::parse_envelope`.
- [ ] 1.4 GREEN: the error-channel and hint-channel checks.
- [ ] 1.5 RED: `tests/conformance_exit_codes.rs` — success envelope + exit 0
      passes; failure envelope + non-zero exit passes; failure envelope + exit
      0 fails; exit 2 with a failure envelope still passes (success/non-success
      comparison, not exact code).
- [ ] 1.6 GREEN: the exit-code check.
- [ ] 1.7 RED: `tests/conformance_report.rs` — one invocation reports every
      violated property (shape + error + hint + exit code all violated), and
      does not stop at the first.
- [ ] 1.8 GREEN: the aggregating entry point (`assert_conforms` /
      `conformance_report`), returning every failure.
- [ ] 1.9 `lib.rs`: `pub mod conformance;`

## 2. Spec (this change)

- [ ] 2.1 Spec delta `specs/conformance/spec.md` (done in this change).
- [ ] 2.2 On archive: deploy `openspec/specs/conformance/spec.md`.
- [ ] 2.3 `.espectacular/conformance/` contract TOMLs mapped to test flags.
- [ ] 2.4 `ah check` green.

## 3. Self-test (genesis)

- [ ] 3.1 The kit runs against genesis's own conformance fixtures (a
      genesis-built success envelope, a genesis-built error envelope) — the
      kit cannot rot.
- [ ] 3.2 A negative self-test: a deliberately malformed envelope is reported
      as non-conforming.

## 4. Downstream (optional, one ticket per repo — filed at approval)

- [ ] 4.1 pretender, vampiro, wai, ddL, specodelic, whisper, spectacular,
      dont: add one conformance test replacing or complementing the local
      compatibility test.

## 5. Tidy

- [ ] 5.1 `just ci` green.
- [ ] 5.2 `versions.ddl.toml` minor bump entry (new public module).
- [ ] 5.3 CHANGELOG entry.
