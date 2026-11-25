# Tasks: add-version-source-abstraction

Tracked as `genesis-cn6` in beads (base_commit `3bffa47`).

Meter: `just ci` and `ah check`.

## 1. Sources (genesis::update_check, feature-gated)

- [ ] 1.1 RED: `tests/update_check_sources.rs` — the crates.io source resolves
      a published version and yields nothing for an unknown crate; the GitHub
      release source resolves a version without a leading `v`; the npm source
      resolves a version; an unreachable publisher yields no version and does
      not panic.
- [ ] 1.2 GREEN: `VersionSource` enum + `latest_version(source, id)` in
      `src/update_check.rs`.
- [ ] 1.3 RED: `tests/update_check_compare.rs` — installed lower → outdated;
      installed equal → current; installed pre-release of the published
      version → current.
- [ ] 1.4 GREEN: the comparison path (`semver`, already optional behind the
      feature).
- [ ] 1.5 RED: `tests/update_check_opt_in.rs` — without the feature, the HTTP
      stack is absent from the dependency graph and no lookup is possible.
- [ ] 1.6 GREEN: keep `ureq`/`semver` optional and feature-gated; no default
      feature change.

## 2. Re-express the crates.io path

- [ ] 2.1 RED: the existing crates.io checks still pass unchanged
      (`check("mytool", version)` behavior, cache TTL, fail-silent, notice
      text).
- [ ] 2.2 GREEN: route `check()` through `VersionSource::CratesIo` with
      identical behavior.

## 3. Spec (this change)

- [ ] 3.1 Spec delta `specs/update-check/spec.md` — publisher abstraction,
      one comparison path, sources stay opt-in (done in this change).
- [ ] 3.2 On archive: deploy `openspec/specs/update-check/spec.md`.
- [ ] 3.3 `.espectacular/update-check/` contract TOMLs mapped to test flags.
- [ ] 3.4 `ah check` green.

## 4. Downstream (separate ticket in the ddL repo — filed at approval)

- [ ] 4.1 dulce-de-leche: `src/installer/upgrade.rs:326,365` — migrate
      `check_latest_version` / `check_all_latest_versions` onto the shared
      sources; keep the npm-CLI routing, the Windows `cmd /C` routing, and the
      `v`-prefix trim; assert identical output for the managed set.

## 5. Tidy

- [ ] 5.1 `just ci` green.
- [ ] 5.2 `versions.ddl.toml` **patch** bump entry (behavior-preserving
      refactor of the crates.io path + new sources behind the same feature).
- [ ] 5.3 CHANGELOG entry naming the supported publishers.
