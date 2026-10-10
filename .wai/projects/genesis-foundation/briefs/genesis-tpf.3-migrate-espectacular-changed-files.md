# Brief: genesis-tpf.3 — Migrate espectacular changed_files_from_git onto genesis::git

**Ticket:** genesis-tpf.3 (P2, child of epic genesis-tpf) · twin: espectacular-o77
**Worktree:** `/var/home/sasha/para/areas/dev/gh/charly/espectacular-git-epic` — branch `feat/genesis-git-migration` (rebased on origin/main @ 10a97e3). Do ALL work here. Do NOT touch the espectacular main checkout.
**Spec (source of truth):** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read before coding.
**Target:** `src/main.rs` `changed_files_from_git` (and any local helpers only it uses).

## Why

Single-function migration (complexity:s) in a dedicated worktree — subagent scope. Lead orchestrator keeps the epic loop and closes the genesis-side ticket at ship.

## Desired outcome

`changed_files_from_git` delegates git invocation to `genesis::git::changed_files` instead of a raw `std::process::Command` git diff. The current contract is silent empty `Vec` on failure — the ticket wants the silent degradation made EXPLICIT: use the documented lossy semantics (genesis::git `changed_files_lossy` if available) or map errors explicitly. Diff-based set excludes untracked while HEAD resolves — spec 'Enumeration set definitions' guarantees this; do not add untracked.

1. **Bump dependency**: `Cargo.toml` `genesis-vibes` → `"0.13"` (genesis::git shipped in v0.13.0); confirm with `cargo tree -p genesis-vibes -i`.
2. Replace the subprocess call with `genesis::git::changed_files(root)` / `changed_files_lossy(root)` — root via `genesis::git::repo_root()` or `repo_root_from(...)`.
3. If you keep the silent-empty behavior, it MUST be via the documented `*_lossy` variant (explicit in code, not hidden). If only the erroring variant exists and the call sites genuinely need silence, use `*_lossy`; if neither fits, STOP and report.
4. Do NOT reimplement any git logic locally. If a needed semantic is missing from genesis::git, STOP and report (follow-up threshold).

## Claim first

`bd update espectacular-o77 --claim` in the espectacular repo (run it in the worktree — bd is repo-scoped). Do NOT close it and do NOT touch genesis-side tickets; lead closes those.

## Red/green sequence

- RED: test asserting the failure path has the chosen explicit contract (error or lossy) — target the migrated behavior first; it must fail pre-migration.
- GREEN: perform the migration; full suite passes.
- Tidy: remove dead local code, `cargo fmt`.

## Exact commands (completion criteria — all must exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i 2>/dev/null | head -1   # 0.13.x
grep -rn "Command::new(\"git\")\|Command::new(\"/usr/bin/git\")" src/   # 0 matches in migrated code
```

## Conventions & gotchas

- Always pass an explicit root to genesis::git ops; never rely on cwd.
- Tests must not mutate process env (parallel test threads).
- Pre-commit hooks run in this worktree; bd export is the lead's job.
- No pushing. Commit locally on `feat/genesis-git-migration` with RED and GREEN as separate commits (conventional messages: `test(...)`, `refactor(...)`).

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash` (single-family — lead-side verification degradation applies).
- Context ceiling: ≤ 60% of window; if exceeded, commit green work, write handoff in final report, stop.

## Follow-up threshold

Shared semantics mismatch, or genesis 0.13 API can't express the op: STOP, leave a note on the ticket twin, commit green work, report — do not reinterpret.
