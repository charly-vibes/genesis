# Brief: genesis-tpf.6 — Migrate whisper private-zone checks onto genesis::git

**Ticket:** genesis-tpf.6 (P3, epic genesis-tpf) · twin: whisper-tlp · worktree `/var/home/sasha/para/areas/dev/gh/charly/whisper-git-epic`, branch `feat/genesis-git-migration` (base bcf0488). Do ALL work here.
**Spec:** `/var/home/sasha/para/areas/dev/gh/charly/genesis/openspec/changes/add-git-interface/specs/git/spec.md` — read first.

## Why

`whisper/src/workspace.rs:602-624`: `git check-ignore -q` single probe + `git ls-files` per file, hand-rolled. Replace with genesis::git `is_ignored` (single-path tri-state) and `tracked`. Current code conflates not-ignored with error (any non-zero exit → Exposed) — that Error→Exposed fail-closed mapping is CORRECT for private-zone detection; keep it at the call site, do NOT move it into genesis.

## Desired outcome

1. Bump `genesis-vibes` → `"0.13"` (confirm 0.13.0; import as `genesis::git::...`).
2. Replace check-ignore probe with `is_ignored(root, path)` tri-state; replace per-file ls-files with `tracked(root, path)`.
3. Preserve fail-closed semantics: Error → treat as Exposed at the whisper call site. Silent-ignore must NOT appear.
4. RED: characterization tests pinning tri-state outcomes (ignored / not-ignored / error) and the fail-closed mapping before migrating. GREEN: migrate; full suite passes.

## Claim first

`bd update whisper-tlp --claim` (in the worktree). Do NOT close tickets.

## Gates (exit 0)

```bash
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo tree -p genesis-vibes -i | head -1   # 0.13.x
```

## Gotchas

- Tests must not mutate process env. No pushing; RED/GREEN separate conventional commits. Hooks (if any) must stay green.

## Spawn model

- Model `openrouter/z-ai/glm-5.3-flash`. ≤60% context; else commit green, handoff, stop.