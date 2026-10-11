# Brief: genesis epic-end — merge z0r branch to main + release v0.13.1

**Repo:** `/var/home/sasha/para/areas/dev/gh/charly/genesis-git-epic` (worktree, branch feat/genesis-git-migration @ pushed). The GENESIS REPO MAIN CHECKOUT (../genesis) is the lead's — do NOT touch it. Do everything from this worktree via git push refspecs.

## Task

genesis-z0r (policy-aware git helpers + path_status XY + changed_files_lossy) is complete on `feat/genesis-git-migration` (pushed, gates green: 136 contract tests via ah check --run-tests, clippy/fmt/test green). Ship it:

1. `git fetch origin && git log --oneline origin/main..HEAD` — confirm branch is strictly ahead (ff). If origin/main moved, rebase and re-run gates (cargo test, clippy --all-targets -- -D warnings, cargo fmt --check, ah check).
2. **Release prep on the branch**: bump `Cargo.toml` version to 0.13.1 (from 0.13.0); stamp CHANGELOG `[0.13.1] — 2026-10-10` moving the [Unreleased] z0r entries (policy-aware helpers, path_status XY, changed_files_lossy, contract TOMLs). Check README.md + docs/getting-started.md version pins — they pin caret "0.13" so a patch bump should not need changes, but VERIFY (doc_sync guard fails CI otherwise; run `just ci` to be sure). Run `just ci` — must be green. Commit: `chore(release): v0.13.1 — genesis::git policy-aware helpers + path_status XY`.
3. **Push branch → remote main**: `git push origin feat/genesis-git-migration:main` (pre-push hooks run from the worktree — proven green; if hooks trip on the release commit, fix and retry; do NOT use --no-verify). Then `git push origin feat/genesis-git-migration` (branch with release commit).
4. **Tag + publish**: `git tag -a v0.13.1 -m "v0.13.1"` on the release commit; `git push origin v0.13.1`. Then `just publish` (or `cargo publish` per justfile). Verify crates.io: `curl -A "genesis-release-verify" https://crates.io/api/v1/crates/genesis-vibes | python3 -c 'import json,sys;print(json.load(sys.stdin)["crate"]["max_stable_version"])'` → 0.13.1. Known benign race: tag-triggered Publish workflow may fail "crate already exists" if just publish won — verify via crates.io, not workflow conclusion.
5. **Notify downstream**: `just notify-downstream 0.13.1` (opens gh announcement issues in the 7 repos).
6. Report: commits, tag, crates.io max_stable_version, CI conclusion on main (gh run list -R charly-vibes/genesis --branch main --limit 2), notify results.

## Rules

- All bd ticket state is the lead's — do not run bd commands.
- Never leave hooks bypassed: no --no-verify, no LEFTHOOK=0 on final commits (fix failures instead).
- If crates.io publish fails on auth or the version can't go out: STOP and report — do not retry more than twice.
- Context ceiling ≤60%: if exceeded, stop after the current step and report state.