# Change: Add `git_hooks` module (shared git hook primitives)

## Why

Three suite tools maintain private, overlapping git-hook primitives:

```
pretender:    install/uninstall .git/hooks/pre-commit shim with ownership
              marker, refuse-foreign-hook guard, chmod 0755
              (pretender/src/main.rs:1196-1266)

wai:          hook owner detection (lefthook/husky/bd/pre-commit/prek sigils),
              core.hooksPath delegation, read-only hook checks
              (wai/src/commands/way/hooks.rs, 516 lines)

espectacular: hook framework detection (Lefthook/Prek/None) + lefthook.yml
              managed-block injection via raw string find/split_at —
              reinventing genesis::managed_block
              (espectacular/src/init.rs:113-281)
```

Two more tools are pure consumers of the *pattern* (dont and testaruda are
wired into other repos' lefthook files via `just check-claims` /
`testaruda select`). Every new suite tool will re-face the same question:
"how do I safely add my gate to this repo's hooks?"

A `genesis::git_hooks` module consolidates the mechanics: marker-based
install/uninstall, owner/framework detection, lefthook managed-block
wiring, and wiring verification. Three consumers ≥ the boundary rule's
two-tool minimum.

## What Changes

### `genesis::git_hooks` — new module

```rust
use genesis::git_hooks::{self, HookName, Owner};

git_hooks::repo_root()?;                              // walk parents to .git
git_hooks::install(root, HookName::PreCommit, MARKER, &script)?;  // refuses foreign
git_hooks::uninstall(root, HookName::PreCommit, MARKER)?;         // removes only own
git_hooks::owner(root, HookName::PreCommit);          // Option<Owner>
git_hooks::framework(root);                           // Lefthook | Prek | Husky | None
git_hooks::lefthook::is_wired(root, stage, "ah check");
git_hooks::lefthook::ensure_wired(root, stage, &block)?;  // idempotent, managed-block based
```

### Donors and what is ported from each

| Donor | Ported | Not ported |
|-------|--------|------------|
| pretender | install/uninstall with marker, refuse-foreign, `repo_root()` | CLI surface (`hooks install` subcommand stays in pretender) |
| wai | `hook_owner` sigil table, `core.hooksPath` resolution, hook-read helpers | `check_git_hooks` way-check (wai-specific `WayCheckEntry` shape) |
| espectacular | `detect_hook_framework`, lefthook block injection — reimplemented on `managed_block` | `InitResult` reporting (tool-specific) |

### Out of scope (follow-up work in consuming repos)

- Migration of pretender's `main.rs` helpers onto the module.
- Migration of espectacular's `init.rs` injection onto the module.
- Migration of wai's `way/hooks.rs` onto the module.
- Any tool-specific gate command (e.g. `ah check`) — those stay in tools.

## Impact

- **Affected specs:** `git-hooks` (new capability)
- **Affected code:** `src/lib.rs` (module list), `src/git_hooks.rs` (new), `Cargo.toml` (no new deps — reuses `managed_block` and existing deps)
- **Beneficiaries:** pretender, espectacular, wai (existing); dont/testaruda indirectly (wiring checks for their gates in host repos)
