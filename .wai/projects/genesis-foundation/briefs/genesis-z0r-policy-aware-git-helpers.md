# Brief: genesis-z0r — genesis::git policy-aware helpers + single-path status XY

**Ticket:** genesis-z0r (P1 — unblocks tpf.5 dont migration) · worktree `/var/home/sasha/para/areas/dev/gh/charly/genesis-git-epic`, branch `feat/genesis-git-migration` (base 203ab3d). Do ALL work here.
**Spec (source of truth):** `openspec/changes/add-git-interface/specs/git/spec.md` (in this repo) — read before coding. If the additions below require spec deltas, extend the spec file in the same change dir (add scenarios + contract TOMLs under `.espectacular` following the existing pattern — see `ah check` findings after adding scenarios).

## Why

The tpf.5 dont migration stopped at the follow-up threshold because genesis::git lacks: (1) policy-aware named helpers, (2) single-path status with porcelain XY. Dont runs inside git hooks (bd hooks) and dispatches on the XY of ONE path.

## Desired outcome (additive only — no breaking changes)

1. **Policy parameter / `*_with_policy` variants** for named helpers `uncommitted_files`, `tracked`, `content_hash`, `committed_content_hash` (and any other named helper that spawns git): default stays `EnvPolicy::Inherit`; add `*_with_policy(root, ..., policy)` or a policy arg so callers can pass `EnvPolicy::StripHookContext`. Internal impl shared — named helpers delegate, zero logic duplication.
2. **`path_status(root, rel) -> Result<Option<PathStatus>, GitError>`** (naming yours): single-path `status --porcelain -- <rel>`. Returns None when clean; Some(untracked) for `?`; Some(dirty) for worktree-modified; Some(staged) for index-modified. Map the XY codes; unmerged states map to dirty (document it).
3. **`changed_files_lossy(root)`** for symmetry with `uncommitted_files_lossy` (tpf.3 finding).
4. TDD: RED tests first for each addition (policy propagation testable by asserting the spawned-env behavior in a hook-like env; XY mapping table test; lossy variant test). GREEN. Tidy.
5. Update `docs/reference/modules.md` git section + CHANGELOG [Unreleased] compat note (new API, additive).

## Conventions & gotchas

- `run()` uses `.current_dir(root)` + relative args — always pass explicit root.
- Tests must not mutate process env (parallel threads) — use the existing `*_with_env` internal-variant pattern from git_hooks (genesis-c64 lesson) for policy/env testing.
- Pre-commit hooks run: `ah check` + fmt/clippy + `testaruda select --safe`; pre-push adds `pretender check` — keep them green.
- `ah check` after adding spec scenarios: deploy contract TOMLs or you'll trip no-toml findings (espectacular stub pattern: `.espectacular/git/<scenario-slug>.toml` mapped to your new test flags).
- No pushing. Commit RED and GREEN separately (conventional messages). Claim the ticket? Already claimed by lead — do NOT close it.

## Gates (all must exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
ah check
```

## Follow-up threshold

If EnvPolicy plumbing requires touching Envelope/evals public API in a breaking way: STOP, note in report, commit green work.

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash`. Context ceiling ≤60%; if exceeded, commit green work, handoff in report, stop.