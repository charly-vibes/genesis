# Brief: genesis-tpf.8 — Migrate wai close.rs uncommitted-files onto genesis::git, fix rename defect

**Ticket:** genesis-tpf.8 (P2, epic genesis-tpf) · twin: wai-jhut · worktree `/var/home/sasha/para/areas/dev/gh/charly/wai-git-epic`, branch `feat/genesis-git-migration` (base a5430f7). Do ALL work here.
**Spec:** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read first.

## Why

`wai/src/commands/close.rs:176` `get_uncommitted_files` uses `line.get(3..)` — for rename lines (`R  old -> new`) this yields a bogus joined string as filename. genesis::git `uncommitted_files` parses porcelain v1 renames correctly (new path). This is a real defect fix, not just a migration.

## Desired outcome

1. Bump `genesis-vibes` to `"0.13"` (confirm `cargo tree -p genesis-vibes -i` → 0.13.0; note lib name is `genesis`, import `genesis::git::...`).
2. Replace `get_uncommitted_files`'s subprocess + manual porcelain parsing with `genesis::git::uncommitted_files(root)`; root via `genesis::git::repo_root()`/`repo_root_from`.
3. RED regression test FIRST: repo with a staged rename (`git mv old new` staged) — assert `get_uncommitted_files` returns the NEW path, not `old -> new`. The test must fail pre-migration.
4. Keep call-site behavior otherwise identical (wai close flow). Delete dead local parsing helpers.
5. Do NOT touch `plugin.rs` / `way/hooks.rs` — those stay on genesis::git_hooks per genesis-orq (--local-only scope semantics preserved there).

## Claim first

`bd update wai-jhut --claim` (bd is repo-scoped; run in the worktree). Do NOT close tickets.

## Red/green

- RED: staged-rename regression test (fails pre-migration); also characterization: untracked included, quoted paths.
- GREEN: migrate; full suite passes (pass-to-pass from baseline).
- Tidy: dead code removal, `cargo fmt`.

## Gates (exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i | head -1   # 0.13.x
```

## Gotchas

- wai's crate name is `wai-cli` on crates.io; tests must not mutate process env (parallel threads). Pre-commit/pre-push hooks run (lefthook + bd) — keep green.
- No pushing. RED/GREEN separate conventional commits.

## Spawn model

- Model `openrouter/z-ai/glm-5.3-flash`. ≤60% context; else commit green, handoff, stop.