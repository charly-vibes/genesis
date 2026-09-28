# Tasks: add-git-hooks

## 1. Foundation — repo root & hooks dir resolution
- [x] 1.1 Write failing tests: `repo_root()` finds root from nested dir; errors outside a repo (use `genesis::fixture` with `with_git_init()`).
- [x] 1.2 Implement `repo_root()` (port from pretender `main.rs`).
- [x] 1.3 Write failing tests: `resolve_hooks_dir()` returns `.git/hooks` by default; honors local `core.hooksPath` when set.
- [x] 1.4 Implement `resolve_hooks_dir()` (port `git_core_hooks_path` from wai `way/hooks.rs`).
- [x] 1.5 Refactor pass: shared error type consistent with `envelope`/`suggestions` conventions.

## 2. Install / uninstall with ownership marker
- [x] 2.1 Write failing tests: install writes hook with marker, 0755 perms; creates hooks dir on demand; refuses foreign hook unmodified; idempotent overwrite of own hook.
- [x] 2.2 Write failing tests: uninstall removes owned hook; no-op when missing; refuses foreign hook unmodified.
- [x] 2.3 Implement `install()` / `uninstall()` (port from pretender, parameterized marker per design D1).
- [x] 2.4 Refactor pass: extract shared read-hook-with-marker helper.

## 3. Detection — owner & framework
- [x] 3.1 Write failing tests: `owner()` returns `Owner::Lefthook`/`Husky`/`Bd`/`PreCommit`/`Prek` per sigil table; most-specific sigil wins (bd-shim-chaining-prek case from wai tests); `None` for missing/unknown.
- [x] 3.2 Implement `owner()` with ordered sigil table (port from wai `hook_owner`, design D3).
- [x] 3.3 Write failing tests: `framework()` → `Lefthook` for lefthook.yml/yaml, `Prek` for prek.toml, `Husky` via sigil, `None` otherwise (port espectacular `detect_hook_framework` + wai delegation tests).
- [x] 3.4 Implement `framework()`.

## 4. Lefthook wiring (managed-block based)
- [x] 4.1 Write failing test: `ensure_wired()` inserts managed block directly after `pre-commit:`; rest of file unchanged.
- [x] 4.2 Write failing tests: creates missing stage section; idempotent when content present; errors when no lefthook config exists (never creates one); malformed config errors instead of mangling.
- [x] 4.3 Implement `ensure_wired(root, Stage, block)` on top of `managed_block` (design D4).
- [x] 4.4 Write failing tests: `is_wired()` true for command in target stage; false when absent / stage missing / no config; false when command is in the other stage only.
- [x] 4.5 Implement `is_wired()` (stage-scoped scan, design D5).

## 5. Integration & hygiene
- [ ] 5.1 Module registered in `src/lib.rs` with doc comment (donor attribution, boundary note: no tool gate strings).
- [ ] 5.2 Guard test: `rg`-style assertion that module source contains no consuming-tool gate commands (`ah check`, `pretender check`, `testaruda select`, `just check-claims`).
- [ ] 5.3 `cargo fmt --check`, `cargo clippy -D warnings`, full `cargo test` green.
- [ ] 5.4 `ah check` passes (spec/test correspondence).

## 6. Follow-up (separate issues, NOT this change)
- [ ] 6.1 pretender: `hooks install|uninstall` onto `genesis::git_hooks`; add pre-push support.
- [ ] 6.2 espectacular: `init.rs` `install_lefthook()` onto `git_hooks::lefthook::ensure_wired`; move `LEFTHOOK_AH_BLOCK` back into espectacular.
- [ ] 6.3 wai: `way/hooks.rs` detection half onto `git_hooks` (keep `WayCheckEntry` shaping local).
