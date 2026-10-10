# Brief: genesis-tpf.5 — Migrate dont git plumbing onto genesis::git (EnvPolicy::StripHookContext)

**Ticket:** genesis-tpf.5 (P2, child of epic genesis-tpf) · twin: dont-9gry
**Worktree:** `/var/home/sasha/para/areas/dev/gh/charly/dont-git-epic` — branch `feat/genesis-git-migration` (fresh from origin/main). Do ALL work here. Do NOT touch the dont main checkout.
**Spec (source of truth):** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read before coding.
**Target:** `src/main.rs` lines ~2744-2841 (repo root, dirty, content hash, hand-rolled env strip).

## Why

dont runs inside git hooks (bd hooks), so it hand-strips GIT_DIR/GIT_INDEX_FILE/GIT_WORK_TREE. genesis::git's `EnvPolicy::StripHookContext` is the opt-in that replaces the hand-rolled strip. complexity:m.

## Desired outcome

Replace the hand-rolled git subprocess block with:
- `genesis::git::repo_root(_from)` for rev-parse --show-toplevel
- `genesis::git::uncommitted_files(root)` for status --porcelain dirty detection
- `genesis::git::content_hash(root, path)` for rev-parse HEAD:file / hash-object
- Run these with `EnvPolicy::StripHookContext` (the documented opt-in) — never the hand-rolled env_remove.

1. **Bump dependency**: `Cargo.toml` `genesis-vibes` → `"0.13"`; confirm `cargo tree -p genesis-vibes -i`.
2. **ANTI-GOAL**: do not change in-repo/out-of-repo degradation behavior — port current tested semantics verbatim. StripHookContext is the mechanism, not a behavior change.
3. Delete the hand-rolled env_remove lines.
4. Do NOT reimplement shared logic locally. Missing semantic → STOP and report.

## Claim first

`bd update dont-9gry --claim` (in the worktree). Do NOT close it; lead closes genesis-side tickets.

## Red/green sequence

- RED: test pinning the current in-repo/out-of-repo degradation + dirty/hash outputs (characterization) — where the migration changes internals only, tests pass; where the explicit env policy matters, add a test that the genesis::git call site uses StripHookContext.
- GREEN: migrate; full suite passes.
- Tidy: remove dead code, `cargo fmt`.

## Exact commands (completion criteria — all must exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i 2>/dev/null | head -1   # 0.13.x
grep -n "GIT_INDEX_FILE\|GIT_WORK_TREE" src/main.rs    # 0 matches in migrated block
```

## Conventions & gotchas

- Always pass an explicit root; never rely on cwd (dont runs from arbitrary hook cwds).
- Tests must not mutate process env (parallel test threads).
- Pre-commit hooks run in this worktree; bd export is the lead's job.
- No pushing. Commit locally on `feat/genesis-git-migration`, RED and GREEN separate commits (conventional messages).

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash` (single-family — lead-side verification degradation applies).
- Context ceiling: ≤ 60% of window; if exceeded, commit green work, write handoff in final report, stop.

## Follow-up threshold

Shared semantics mismatch (esp. dirty detection or content hash vs hash-object), or API can't express the op: STOP, note on twin, commit green work, report — do not reinterpret.
