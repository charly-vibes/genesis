# fix-hooks-dir-scopes

## Why

`resolve_hooks_dir()` only honors repo-local `core.hooksPath`. When a user
has a global `core.hooksPath` configured (e.g. a `~/.git-hooks` lefthook
shim), hooks installed by downstream tools are silently never invoked —
git uses the global path and the hook in `.git/hooks` is bypassed.
Found in the wild via pretender-15w (2026-09-29); reported as
genesis-c64 / gh issue #12.

## What Changes

- `resolve_hooks_dir()` resolves `core.hooksPath` across all scopes
  (local → global → system), matching git's own precedence, instead of
  `--local` only.
- New `effective_hooks_dir()` returns the resolved directory **plus** the
  scope it was found in, so doctor-style checks can warn when a hook was
  installed at a different scope than the one git will use.
- Empty-string `core.hooksPath` (which disables hooks in git) is surfaced
  distinctly as `HooksDirScope::Disabled` instead of silently falling
  back to `.git/hooks`.
- Consumers of `resolve_hooks_dir()` are unaffected: relative values
  still resolve against the repo root and unset config still yields
  `.git/hooks`.

## Impact

- Affected specs: `git-hooks` (modified `Marker-based hook installation`
  scenario; new `Effective hooks directory resolution` requirement)
- Affected code: `src/git_hooks.rs` (resolution functions + new types)
- No breaking API change: `resolve_hooks_dir` keeps its signature and
  documented behavior for local-scope users; the empty-string case keeps
  its old fallback in `resolve_hooks_dir` and is only surfaced distinctly
  through the new API.
