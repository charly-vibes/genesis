# Brief: genesis-tpf.2 — Migrate testaruda change.rs onto genesis::git

**Ticket:** genesis-tpf.2 (P2, child of epic genesis-tpf) · twin: testaruda-licv
**Worktree:** `/var/home/sasha/para/areas/dev/gh/charly/testaruda-git-epic` — branch `feat/genesis-git-migration` (base origin/main @ 6997713). Do ALL work here. Do NOT touch the testaruda main checkout.
**Spec (source of truth):** genesis repo `openspec/changes/add-git-interface/specs/git/spec.md` at `/var/home/sasha/para/areas/dev/gh/charly/genesis/` — read it before coding.
**Target file:** `src/change.rs` (all logic lives in `ChangeSet::from_diff` + `parse_porcelain_line`; tests are the `#[cfg(test)]` mod at the bottom).

## Why

This ticket is isolated single-file migration work (complexity:s) in a dedicated worktree — ideal subagent scope — while the lead orchestrator keeps the epic loop running and prepares the other 5 donor worktrees. Inline implementation in the lead session would violate the orchestrator canon (Invariant 1).

## Desired outcome

`ChangeSet::from_diff` in `testaruda/src/change.rs` delegates all git invocation to `genesis::git` instead of raw `std::process::Command` git calls. Existing tests keep passing (pass-to-pass), error semantics are preserved or improved (miette Result, not silent-empty).

1. **Bump dependency**: `Cargo.toml` `genesis-vibes = "0.10"` → `"0.13"` (genesis::git shipped in v0.13.0). Check `cargo update -p genesis-vibes` resolves to 0.13.x.
2. **Range diff** (`base`+`head` set): replace `git diff --name-only b h` subprocess with `genesis::git::changed_files_between(root, b, h)` — obtain root via `genesis::git::repo_root_from(&std::env::current_dir().unwrap_or_default())` or `repo_root()`. Map `GitError` → miette error preserving the "git diff exited with N + stderr" shape (include `code` and `stderr` from `GitError::Git`). `from_revisions: true` must still be set.
3. **Uncommitted path**: replace `git status --porcelain` subprocess + local `parse_porcelain_line` with `genesis::git::uncommitted_files(root)` (includes untracked, uses shared `parse_porcelain`). Rename/copy/new-path semantics of the shared parser are the same contract — existing parse tests must pass against the shared parse (port the test cases into a test asserting shared-parser behavior where the local tests exercised it).
4. **Keep** the explicit-file-list branch untouched.
5. **Delete local `parse_porcelain_line`** (or re-point any external caller to `genesis::git::parse_porcelain` — `grep -rn parse_porcelain src/ tests/` first; currently only change.rs and commands.rs use from_diff).
6. Do NOT reimplement any git logic locally. If a needed semantic is missing from genesis::git, STOP and report (follow-up threshold).

## Out of scope

- No changes to `src/commands.rs` call site (signature unchanged).
- No new git features, no spec edits, no genesis repo changes.
- No pushing, no bd close of the genesis-side ticket (lead does that).

## Red/green/refactor sequence

- RED: write a test asserting `from_diff` failure path returns a miette error carrying the git exit code/stderr (run in a temp dir that is NOT a repo) — currently the subprocess path may return a different error shape; make the test target the migrated contract first.
- RED: port one rename + one quoted-path/untracked porcelain test to run against the shared `parse_porcelain` (it already passes — this is a characterization port; if any local behavior differs from shared parser, STOP and report the diff instead of papering over it).
- GREEN: perform the migration; all existing tests in change.rs and the full suite pass.
- Tidy: remove dead code (`parse_porcelain_line`), `cargo fmt`.

## Exact commands (completion criteria — all must exit 0)

```bash
cargo test --quiet                          # full suite, pass-to-pass from 269-test baseline
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i 2>/dev/null | head -1   # confirms 0.13.x in graph
```

## Conventions & gotchas

- genesis repo-root walk handles worktrees and `.git` dirs; `run()` uses `.current_dir(root)` with relative args — always pass a root, never rely on cwd.
- `uncommitted_files` errors when not in a repo (GitError::NotInRepo) — map to miette with a helpful message; do NOT silently degrade (ticket says preserve error semantics).
- Pre-commit hooks run in this worktree; `bd` export is the lead's job — skip `git add -f .beads` (the claim step already staged it; include it in your first commit is fine but not required).
- Tests must not mutate process env (parallel test threads).

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash` (single-family — lead-side verification degradation applies).
- Context ceiling: ≤ 60% of the model window; if exceeded, commit green work, write a handoff in the final report, stop.

## Follow-up threshold

If the shared parser semantics differ from local behavior on renames/quoting, or genesis 0.13 API can't express a needed op: STOP, leave a note in the ticket-twin, commit green work, report — do not reinterpret.
