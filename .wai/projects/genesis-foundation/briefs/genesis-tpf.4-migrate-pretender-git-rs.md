# Brief: genesis-tpf.4 — Migrate pretender src/git.rs onto genesis::git, drop git2

**Ticket:** genesis-tpf.4 (P2, child of epic genesis-tpf) · twin: pretender-4hc
**Worktree:** `/var/home/sasha/para/areas/dev/gh/charly/pretender-git-epic` — branch `feat/genesis-git-migration` (rebased on origin/main @ 831fcc4). Do ALL work here. Do NOT touch the pretender main checkout.
**Spec (source of truth):** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read before coding.
**Target:** `pretender/pretender/src/git.rs` (`staged_files`, `diff_base_files`) + `pretender/pretender/Cargo.toml`.

## Why

This is the only git2 link in the whole tool suite (complexity:m). Removing it removes the sole native dependency; the suite then runs on one subprocess substrate.

## Desired outcome

1. Reimplement `staged_files` and `diff_base_files` on `genesis::git` subprocess ops (e.g. staged set + changed_files_between as appropriate) — preserve staged-only vs diff-base set semantics exactly, including the enumeration-set guarantees (staged excludes untracked; diff-base excludes untracked while base resolves).
2. **Drop git2** from `pretender/pretender/Cargo.toml` — ANTI-GOAL: zero git2 references when closed. No shim, no feature-flagged old implementation.
3. `gitignore_walk` test fixture uses git2 `Repository::init` — switch to plain `git init` via `std::process::Command` (test-only setup is fine) or a genesis fixture.
4. Bump `genesis-vibes` to `"0.13"`; confirm via `cargo tree -p genesis-vibes -i`.
5. Do NOT reimplement shared logic locally. If genesis::git can't express a needed op, STOP and report.

## Claim first

`bd update pretender-4hc --claim` (in the worktree — bd is repo-scoped). Do NOT close it; lead closes genesis-side tickets.

## Red/green sequence

- RED: characterization tests pinning current staged/diff-base set semantics against non-repo, unborn-HEAD, and typical staged+untracked fixtures — these must fail or expose gaps pre-migration where behavior changes.
- GREEN: migrate; full suite passes (pass-to-pass from current baseline).
- Tidy: delete dead git2-era code, `cargo fmt`.

## Exact commands (completion criteria — all must exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
grep -rn "git2" pretender/pretender/Cargo.toml pretender/pretender/src/   # 0 matches
cargo tree -p genesis-vibes -i 2>/dev/null | head -1   # 0.13.x
```

## Conventions & gotchas

- Always pass an explicit root to genesis::git ops; never rely on cwd.
- Tests must not mutate process env (parallel test threads).
- Pre-commit hooks run in this worktree; bd export is the lead's job.
- No pushing. Commit locally on `feat/genesis-git-migration`, RED and GREEN as separate commits (conventional messages).

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash` (single-family — lead-side verification degradation applies).
- Context ceiling: ≤ 60% of window; if exceeded, commit green work, write handoff in final report, stop.

## Follow-up threshold

Shared semantics mismatch on enumeration sets, or genesis 0.13 API can't express staged/diff-base: STOP, note on twin, commit green work, report — do not reinterpret.
