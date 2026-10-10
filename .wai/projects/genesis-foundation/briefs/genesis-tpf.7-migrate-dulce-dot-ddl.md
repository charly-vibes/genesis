# Brief: genesis-tpf.7 — Migrate dulce dot_ddl tracked check onto genesis::git

**Ticket:** genesis-tpf.7 (P3, epic genesis-tpf) · twin: DDL-u8u · worktree `/var/home/sasha/para/areas/dev/gh/charly/dulce-git-epic`, branch `feat/genesis-git-migration` (base 723f81d). Do ALL work here.
**Spec:** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read first.

## Why

`dulce-de-leche/src/dot_ddl.rs:808`: `git ls-files --error-unmatch <single file>` subprocess. Replace with genesis::git `tracked`. Preserve the `-C <parent>` invocation semantics via explicit root (`repo_root_from` / `repo_root()`), never cwd reliance.

## Desired outcome

1. Bump `genesis-vibes` → `"0.13"` (confirm 0.13.0; import `genesis::git::...`).
2. Replace the ls-files probe with `tracked(root, path)`; map not-tracked vs error exactly as today (port the current tested degradation verbatim — ANTI-GOAL: no behavior change).
3. RED: characterization test pinning tracked/untracked/non-repo outcomes pre-migration. GREEN: migrate; full suite passes.

## Claim first

`bd update DDL-u8u --claim` (in the worktree). Do NOT close tickets.

## Gates (exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i | head -1   # 0.13.x
```

## Gotchas

- Tests must not mutate process env. No pushing; RED/GREEN separate conventional commits.

## Spawn model

- Model `openrouter/z-ai/glm-5.3-flash`. ≤60% context; else commit green, handoff, stop.