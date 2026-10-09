## 1. Module (genesis, no new dependencies)
- [ ] 1.1 RED: `tests/git_repo_root.rs` — walk-based root detection: normal repo, worktree (`.git` file), inside `.git` dir, nested subdir, not-in-repo error. Assert independence from `GIT_DIR` env.
- [ ] 1.2 GREEN: `repo_root` / `repo_root_from` in `src/git.rs`; alias `git_hooks::repo_root_from` onto it (no behavior change).
- [ ] 1.3 RED: `tests/git_runner.rs` — canonical `run()`: env policy `Inherit` keeps `GIT_DIR`; `StripHookContext` removes `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` (dont's behavior); non-zero exit surfaces as typed `GitError::Git { args, code, stderr }`; git-missing (spawn failure) as `GitError::Spawn`.
- [ ] 1.4 GREEN: `run()` + `EnvPolicy` + `GitError` (thiserror, miette-compatible).
- [ ] 1.5 RED: `tests/git_changed_files.rs` — staged-only vs HEAD, between two revs, unborn HEAD (fresh repo → untracked fallback, not empty set), empty repo error shape, lossy variant returns empty set on git failure; untracked asymmetry (uncommitted includes untracked, changed_files excludes untracked while HEAD resolves).
- [ ] 1.6 GREEN: `changed_files` / `changed_files_between` / `uncommitted_files` / `uncommitted_files_lossy`.
- [ ] 1.7 RED: `tests/git_porcelain.rs` — porcelain v1 parse: plain `XY path`, rename `R  old -> new` (returns new path), copy `C`, quoted paths with `core.quotePath`, non-ASCII names, empty status output.
- [ ] 1.8 GREEN: `parse_porcelain` (donor: testaruda `change.rs:91`; supersede wai `close.rs` `line.get(3..)` behavior).
- [ ] 1.9 RED: `tests/git_tracked.rs` — `tracked` via `ls-files` (committed vs never-added path), `check-ignore` tri-state for a single path (exit 0 / 1 / 128+ mapped to Ignored / NotIgnored / Error), `content_hash` via `hash-object` (deterministic) and `rev-parse HEAD:path` for committed content.
- [ ] 1.10 GREEN: `tracked` / `is_ignored` / `content_hash`.
- [ ] 1.11 `lib.rs`: `pub mod git;`

## 2. Spec (this change)
- [ ] 2.1 Spec delta + deployed spec `openspec/specs/git/spec.md`.
- [ ] 2.2 `.espectacular/git/` contract TOMLs mapped to test flags (one per scenario).
- [ ] 2.3 `ah check` green.

## 3. Downstream migrations (separate tickets, one per repo — filed at approval)
- [ ] 3.1 testaruda: `change.rs` onto `changed_files_between` / `uncommitted_files` (error semantics preserved).
- [ ] 3.2 espectacular: `changed_files_from_git` onto `changed_files` behind its documented lossy wrapper.
- [ ] 3.3 pretender: `src/git.rs` onto shared module; drop `git2` dependency.
- [ ] 3.4 dont: `main.rs:2744+` onto repo root / dirty / content hash with `EnvPolicy::StripHookContext`.
- [ ] 3.5 whisper: `workspace.rs` private-zone checks onto `is_ignored` / `tracked`.
- [ ] 3.6 dulce: `dot_ddl.rs` tracked check onto `tracked`.
- [ ] 3.7 wai: `close.rs` onto `uncommitted_files` (rename-line defect fixed); `plugin.rs`/`way/hooks.rs` stay on `git_hooks` per genesis-orq (note: preserve `--local`-only scope semantics).

## 4. Tidy
- [ ] 4.1 `just ci` green (fmt, clippy, test, docs, build-release, aix-check).
- [ ] 4.2 `versions.ddl.toml` minor bump entry (new public module).
