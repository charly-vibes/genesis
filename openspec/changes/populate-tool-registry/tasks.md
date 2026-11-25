# Tasks: populate-tool-registry

Tracked as `genesis-dyn` in beads (base_commit `3bffa47`).

Meter: `just ci` and `ah check`.

## 1. Schema (genesis::discovery)

- [ ] 1.1 RED: `tests/discovery_states.rs` — a declared tool whose marker is
      absent is reported as declared and not installed, and discovery does not
      fail; a present marker reports declared and installed.
- [ ] 1.2 GREEN: the declared/installed distinction in `src/discovery.rs`.
- [ ] 1.3 RED: `tests/discovery_detectors.rs` — file, directory, and command
      detectors resolve; an unknown detector kind is reported as invalid and
      never as installed.
- [ ] 1.4 GREEN: the `command` detector kind; unknown kinds reported as
      invalid.
- [ ] 1.5 RED: `tests/discovery_invalid.rs` — an entry without a detector
      path is invalid; an unknown detector kind is invalid (a distinct state
      from "not installed"); a malformed manifest yields a report, not an
      error and not a panic.
- [ ] 1.6 GREEN: entry validation and non-fatal reporting; the state machine
      gains a third outcome (invalid) alongside installed / not-installed —
      today both an empty path and an unknown kind collapse to "not detected"
      (`src/discovery.rs:128-138`).

## 2. Fleet registry (this repo)

- [ ] 2.1 RED: `tests/discovery_fleet.rs` — the reference manifest contains an
      entry for each Rust CLI in the suite, each with a description and a
      detector.
- [ ] 2.2 GREEN: populate `.genesis/tools.toml` for the fleet — wai, dont,
      dulce-de-leche, testeruda, espectacular, pretender, specodelic,
      whisper, vampiro, podras, evallerina, bajan, sad, gently, kaya, wild,
      sunshine, funkalization.
- [ ] 2.3 Add the `.espectacular` detector entry (closes `genesis-lu6`).
- [ ] 2.4 Re-scan the `testaruda` repo and choose its detector from evidence
      (the audit could not read that tree); keep the existing file-marker
      entry until then.

## 3. Spec (this change)

- [ ] 3.1 Spec delta `specs/discovery/spec.md` (done in this change).
- [ ] 3.2 On archive: deploy `openspec/specs/discovery/spec.md`.
- [ ] 3.3 `.espectacular/discovery/` contract TOMLs mapped to test flags.
- [ ] 3.4 `ah check` green.

## 4. Downstream (one ticket per tool repo — filed at approval)

- [ ] 4.1 Each tool repo declares its own entry in its `.genesis/tools.toml`,
      matching the reference copy in this repo.

## 5. Tidy

- [ ] 5.1 `just ci` green.
- [ ] 5.2 `versions.ddl.toml` **minor** bump entry (schema change: new
      detector kind + a new invalid state).
- [ ] 5.3 CHANGELOG entry naming the declared/installed split.
