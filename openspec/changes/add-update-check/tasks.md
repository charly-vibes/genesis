## 1. Module (genesis, feature `update-check`)
- [x] 1.1 `Cargo.toml`: optional `ureq` + `semver`, `[features] update-check`, test recipe `--all-features`.
- [x] 1.2 RED: tests/update_check.rs — hermetic local-TCP-server suite (no crates.io access).
- [x] 1.3 RED: tests/update_check_env.rs — env-skip tests in a separate binary (env vars are process-global).
- [x] 1.4 GREEN: `check`/`check_with`/`notice`/`cache_path`, cache read/short-circuit, fetch, yanked+prerelease filter, rate-limit TTL doubling, atomic best-effort cache writes.
- [x] 1.5 lib.rs: `#[cfg(feature = "update-check")] pub mod update_check;`

## 2. Spec (this change)
- [x] 2.1 Spec delta + deployed spec `openspec/specs/update-check/spec.md`.
- [x] 2.2 `.espectacular/update-check/` contracts mapped to test flags.
- [x] 2.3 `ah check` green.

## 3. Downstream wiring (separate tickets, one per repo)
- [ ] 3.1 wai: wire `check()` after command output in its binary; reuse in doctor/version.
- [ ] 3.2 pretender: same.
- [ ] 3.3 testaruda: same.

## 4. Tidy
- [ ] 4.1 `just ci` green (fmt, clippy, test, docs, build-release, aix-check).