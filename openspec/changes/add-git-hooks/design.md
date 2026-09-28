# Design: `git_hooks` module

## Context

Three donors, three different maturity levels of the same primitives:

- **pretender** (`pretender/src/main.rs:1196-1266`): full
  install/uninstall lifecycle with ownership marker — the only donor with
  a *write* path. No pre-push support.
- **wai** (`wai/src/commands/way/hooks.rs`, 516 lines): the richest
  *read* path — sigil-based owner detection (lefthook, husky, bd,
  pre-commit, prek), `core.hooksPath` delegation, chaining-shim handling
  (e.g. bd hook that chains prek passes). Read-only; never installs.
- **espectacular** (`espectacular/src/init.rs:113-281`): framework
  detection + lefthook.yml injection, but implemented with raw
  `str::find`/`split_at` surgery and a hardcoded `LEFTHOOK_AH_BLOCK`.

## Goals / Non-Goals

- Goals: one module covering install/uninstall (any hook name), owner and
  framework detection, lefthook wiring via `managed_block`, wiring checks.
- Non-Goals: CLI surface (stays in tools); running hooks; supporting
  husky/pre-commit-framework *installation* (detection only); creating
  lefthook configs from scratch.

## Decisions

### D1: Marker-based ownership, parameterized (from pretender)

Pretender's `PRE_COMMIT_HOOK_MARKER` pattern generalizes: every install
takes a `marker: &str` parameter (tools embed their own name, e.g.
`# Installed by Pretender.`). Same rule on uninstall. This keeps tool
identity out of genesis (boundary rule) while preserving the
refuse-foreign-hook safety that all donors rely on.

*Alternative considered:* a registry of known tools with fixed markers —
rejected: couples genesis to consumer release cycles.

### D2: `core.hooksPath` respected on install, resolved on read (from wai)

Wai resolves `git config --local core.hooksPath` when reading hooks;
espectacular ignores it. The module resolves the hooks directory once
(`resolve_hooks_dir(root)`) and uses it for install, uninstall, and read
paths. This also matches wai's test
`lefthook_yml_with_delegated_hooks_path_detects_lefthook`.

*Alternative considered:* installing only to `.git/hooks` and erroring on
delegation — rejected: silently wrong install target is worse than
following git's own semantics.

### D3: Owner detection is a sigil table, most-specific-first (from wai)

Port wai's ordered table (`lefthook`, `husky`, `bd`, `pre-commit`, …)
into an enum `Owner { Lefthook, Husky, Bd, PreCommit, Prek }`. Order
matters (wai's tests: a bd hook that chains prek must report `bd`, not
`prek`). Extensible by appending table entries.

### D4: Lefthook injection rebuilt on `managed_block` (replaces espectacular)

Espectacular's `install_lefthook()` does `find("pre-commit:")` +
`split_at` — fragile (no anchor handling, no idempotence check beyond
substring search, duplicates `managed_block`). The module wraps
`managed_block` injection anchored at the stage key (`pre-commit:` /
`pre-push:`), giving marker-tagged, idempotent blocks for free. The
`LEFTHOOK_AH_BLOCK` content moves back to espectacular at migration time.

*Alternative considered:* full YAML parse/emit — rejected: adds a YAML
dependency and reformats users' files; lefthook configs in the suite are
small and conventionally structured.

### D5: Wiring check = stage-scoped section scan (from wai doctor)

Primary consumer: wai's doctor (`checks_basic.rs:603-651`), which verifies
"is gate X wired into stage Y" for both stages with distinct fix hints.
Espectacular's need is narrower: `init.rs` does a substring idempotence
check and its doctor (`doctor.rs:342-359`) only checks framework presence.
The idempotence check is a special case of `is_wired` ("already true"), so
espectacular's init can adopt `is_wired` at migration time.

`lefthook::is_wired()` scans only the target stage's section — a command
wired into pre-push must not satisfy a pre-commit check. Section
semantics: the anchor is the stage key at column 0 (e.g. `^pre-commit:`);
the section extends to the next column-0 key or EOF. The module returns
the boolean; tools own the fix-hint text.

### D5b: `Stage` vs `HookName`

`HookName` (install/uninstall/owner) covers hook file names;
`Stage` (lefthook wiring) is the lefthook-config subset (`pre-commit`,
`pre-push`). Kept separate because lefthook keys are config vocabulary,
not file names — conversion happens only if a consumer needs it.

### D6: Ambiguous configs (edge policy)

- **Both `lefthook.yml` and `lefthook.yaml` present:** `.yml` wins
  (espectacular's existing precedence); the other is ignored.
- **Empty (0-byte) hook file:** treated as a foreign hook on install
  (refused until the user removes it) — matches pretender's marker check;
  wai's `hook_exists_nonempty`-style "empty = absent" is rejected because
  silently overwriting a file the user created is worse than refusing.
- **Unanchorable lefthook config** (quoted keys, unrecognized structure):
  `ensure_wired()` errors without modifying the file (specced in the
  Lefthook managed-block wiring requirement).

## Risks / Trade-offs

- **Sigil false positives** — a user hook mentioning "lefthook" in a
  comment is misattributed. Accepted: wai has lived with this; severity
  is a wrong advisory message, not data loss.
- **Premature abstraction** — module is built from three real donors, so
  demand is proven; the risk is over-generalizing. Mitigation: only hook
  names/stages used by donors (`pre-commit`, `pre-push`) are enum
  variants; `HookName::Other(&str)` is the escape hatch.

## Migration Plan

Not in this change. Follow-up issues per consumer
(pretender `hooks install|uninstall`, espectacular `init.rs`, wai
`way/hooks.rs`) switch onto the module and delete their private copies.
Each is independent; genesis ships first.

### D7: `framework()` reports the single detected framework, including husky (decided)

`framework()` reports the repository's one hook-management framework
generically: `Framework::{Lefthook, Prek, Husky, None}`. The repository
is assumed to use at most one framework (espectacular's assumption,
confirmed as the suite convention). Husky has no root config; it is
detected via the hook-file sigil (reusing `owner()`'s table) when no
lefthook/prek config exists. If signals coexist — rare, defensive only —
report deterministically by precedence: Lefthook, then Prek, then Husky.

Rationale for including `Husky` (user decision, resolves the former open
question): `Framework::None` on a husky repo is informationally lossy —
espectacular's doctor would claim "no supported framework detected" on a
repo that clearly has hook management. `Husky` gives consumers a
distinguishable "detected but not wirable" state; `ensure_wired()`
remains lefthook-only.

## Open Questions

None — the husky question is resolved by D7.
