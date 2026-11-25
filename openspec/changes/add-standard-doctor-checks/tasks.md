# Tasks: add-standard-doctor-checks

Tracked as `genesis-ykp` in beads (base_commit `3bffa47`).

Meter: `just ci` and `ah check`.

## 1. Result vocabulary (genesis::doctor)

- [ ] 1.1 RED: `tests/doctor_not_applicable.rs` — a result state for "not
      applicable" exists; it serializes distinctly from healthy/advisory and
      from a failure; a report containing one renders it distinctly.
- [ ] 1.2 GREEN: extend the result type in `src/doctor/check.rs` with the
      not-applicable state; the existing severity vocabulary is unchanged.

## 2. Standard battery (genesis::doctor::standard)

- [ ] 2.1 RED: `tests/doctor_standard_config.rs` — config path exists and
      parses → valid; config path missing → failure naming the file; config
      path not configured → not-applicable.
- [ ] 2.2 GREEN: the config check.
- [ ] 2.3 RED: `tests/doctor_standard_repo.rs` — inside a Git repository →
      healthy; outside → failure naming the condition.
- [ ] 2.4 GREEN: the Git-repository check (uses `genesis::git::repo_root`).
- [ ] 2.5 RED: `tests/doctor_standard_blocks.rs` — a configured block whose
      content matches → healthy; a drifted block → failure naming the block; no
      configured blocks → not-applicable.
- [ ] 2.6 GREEN: the managed-block drift check (uses `genesis::managed_block`).
- [ ] 2.7 RED: `tests/doctor_standard_ci.rs` — configured workflow present →
      healthy; missing → failure naming the workflow; not configured →
      not-applicable.
- [ ] 2.8 GREEN: the CI-workflow check.
- [ ] 2.9 RED: `tests/doctor_standard_battery.rs` — the battery enumerates
      exactly the four standard checks; a report from a partially configured
      battery distinguishes applicable from not-applicable rows.
- [ ] 2.10 GREEN: `src/doctor/standard.rs` — the battery constructor.
- [ ] 2.11 `src/doctor.rs`: `pub mod standard;`

## 3. Status bridge

- [ ] 3.1 RED: `tests/doctor_status_bridge.rs` — a doctor runner registered
      with the shared status builder contributes its findings; a
      not-applicable row produces no failure item; severity is preserved.
- [ ] 3.2 GREEN: the bridge carries the new state (extend
      `src/status/bridge.rs`).

## 4. Spec (this change)

- [ ] 4.1 Spec delta `specs/doctor/spec.md` (done in this change).
- [ ] 4.2 On archive: deploy `openspec/specs/doctor/spec.md`.
- [ ] 4.3 `.espectacular/doctor/` contract TOMLs mapped to test flags.
- [ ] 4.4 `ah check` green.

## 5. Downstream (optional, one ticket per repo — filed at approval)

- [ ] 5.1 wai, dont, ddL, vampiro, pretender, espectacular, specodelic,
      whisper, gently: adopt the standard checks their tool configures,
      keeping their domain checks.

## 6. Tidy

- [ ] 6.1 `just ci` green.
- [ ] 6.2 `versions.ddl.toml` minor bump entry (new public module).
- [ ] 6.3 CHANGELOG entry.
