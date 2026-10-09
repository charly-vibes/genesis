# Change: Add `git` module (shared read-only git plumbing)

## Why

Eight suite tools shell out to `git` (or link `git2`) and re-implement the
same plumbing with visible drift (audited 2026-10-09, tracked as genesis-tpf;
reviewed via Rule-of-5 with TypeSafe verification):

- **Repo-root detection** — dont (`main.rs:2762`, `rev-parse --show-toplevel`),
  specodelic (`packs.rs:364`, same), wai (`plugin.rs:659-673`,
  `rev-parse --git-dir` / `--git-common-dir`) vs genesis
  (`git_hooks::repo_root_from`, walk-based). Two mechanisms, four
  implementations; they differ inside `.git` dirs, under worktrees, and
  when `GIT_DIR` is set (wai's variant asks for `--git-dir` instead of
  `--show-toplevel`, a third answer for the same question).
- **Changed / uncommitted files** — testaruda (`change.rs:40,68`), espectacular
  (`changed_files_from_git`), pretender (`git.rs`: `staged_files`,
  `diff_base_files` via **git2**), wai (`close.rs:176`). Two conflicting
  failure conventions: espectacular and wai silently return an empty `Vec`,
  testaruda returns errors.
- **Porcelain parsing** — testaruda (`change.rs:91` parse_porcelain_line)
  handles rename/copy lines; wai `close.rs` uses `line.get(3..)`, which for a
  rename line yields the bogus string `old -> new` as one filename.
- **Tracked / ignored checks** — whisper (`workspace.rs:602-624`,
  `check-ignore -q`, `ls-files`), dulce (`dot_ddl.rs:808`,
  `ls-files --error-unmatch`).
- **Content hashing** — dont (`main.rs:2779-2841`, `rev-parse HEAD:file` and
  `hash-object`).
- **Substrate split** — pretender is the only tool linking `git2` (confirmed
  recursively over all suite `Cargo.toml`s); every other tool spawns the
  `git` binary.

All of these are read-only queries consumed by two or more tools — the
boundary rule is satisfied with room to spare. The write paths (`add`,
`commit`, `init`) stay out: only wai uses them, and they are domain action,
not plumbing. Hook wiring stays in `git_hooks` (spec `git-hooks`).

## What Changes

### `genesis::git` — subprocess-only, read-only

```rust
use genesis::git::{repo_root, changed_files, uncommitted_files, is_ignored, content_hash};

let root = repo_root()?;                          // walk-based, worktree-safe
let files = changed_files(root)?;                 // worktree vs HEAD, unborn-HEAD aware
let files = changed_files_between(root, "a", "b")?;
let files = uncommitted_files(root)?;             // porcelain v1, rename-aware
let files = uncommitted_files_lossy(root);        // documented silent-empty variant
let ignored = is_ignored(root, &path)?;           // single path; Ignored/NotIgnored/Error
let sha = content_hash(root, &path)?;             // hash-object / HEAD:path
```

Decisions baked in from the review:

- **Subprocess substrate only.** No `git2`/`gitoxide` dependency; the module
  spawns `git`. Pretender drops its `git2` dep in a separate migration ticket.
- **One runner, opt-in env hygiene.** A canonical `run()` helper exists, but
  dont's `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` stripping is an *opt-in*
  policy (`EnvPolicy::StripHookContext`), never the default — dont needs it
  because it executes inside git hooks; tools that legitimately inherit
  `GIT_DIR` context must keep it.
- **Declared failure semantics.** Query ops return `Result<_, GitError>`.
  Lossy variants (`*_lossy`) that silently degrade to empty collections are
  explicit, named, and documented — the silent-empty convention of
  espectacular/wai-close is preserved only where callers opt into it.
- **Repo-root by filesystem walk** (promoted from `git_hooks::repo_root_from`,
  which keeps delegating). Documented divergences from
  `rev-parse --show-toplevel`: works where `.git` is a file (worktrees),
  works inside `.git` dirs, ignores `GIT_DIR` env. `git_hooks` behavior is
  unchanged.
- **Porcelain v1 parsing is part of the contract.** Statuses XY, rename/copy
  `old -> new` (new path taken), quoted paths (`core.quotePath`), non-ASCII
  names. Fixes the wai `close.rs` rename defect at the donor, in wai's own
  migration ticket.
- **Unborn-HEAD fallback.** On a fresh repo `git diff --name-only HEAD` exits
  128 ("ambiguous argument HEAD"); `changed_files` SHALL detect this and fall
  back to untracked-file enumeration instead of returning a silent empty set.
- **check-ignore tri-state, single path.** `is_ignored` takes exactly one
  path and distinguishes Ignored (exit 0), NotIgnored (exit 1), and Error
  (exit 128+) — whisper's current code conflates not-ignored with error,
  and multi-path `check-ignore` calls exit 0 when *any* path is ignored,
  which would poison the tri-state.
- **Declared enumeration sets.** `uncommitted_files` SHALL include
  untracked entries (donor: testaruda, whose test asserts `??` entries);
  `changed_files` SHALL exclude untracked files when `HEAD` resolves
  (diff-based semantics the espectacular/pretender donors depend on). The
  unborn-HEAD fallback is the stated exception, not a hidden asymmetry.

### Out of scope

- Hook discovery/install/uninstall — owned by `git_hooks` (spec `git-hooks`).
  Migrations of wai `way/hooks.rs` and pretender are genesis-orq / genesis-q4k
  and must preserve wai's deliberate `--local`-only scope semantics
  (`hooks.rs:33-50`); those tickets may add a scope parameter to `git_hooks`
  but do not belong to this change.
- Write paths (`git init/add/commit`) — stay in wai.
- `git log`-style history queries — wai-only today; revisit when a second
  consumer appears.

## Impact

- **Affected specs**: new `git` spec.
- **Affected code**: `src/git.rs` (new), `src/lib.rs` (module decl),
  `src/git_hooks.rs` (repo_root_from re-exported/aliased, no behavior change).
- **Downstream migrations** (separate tickets, one per repo, each preserving
  the donor's tested semantics):
  - testaruda `change.rs` → `changed_files_between` / `uncommitted_files`
  - espectacular `changed_files_from_git` → `changed_files` + lossy wrapper
  - pretender `git.rs` → shared module, `git2` dep dropped
  - dont `main.rs:2744+` → repo root, dirty, content hash, env policy
  - whisper `workspace.rs` → `is_ignored` / `tracked`
  - dulce `dot_ddl.rs` → `tracked`
  - wai `close.rs` → `uncommitted_files` (rename defect fixed)
- **New tickets**: one bd issue per downstream migration, filed at approval.
