# Brief: genesis-tpf.1 — Implement `genesis::git` module

**Ticket:** genesis-tpf.1 (P1, child of epic genesis-tpf)
**Branch:** main (no parallel agent in genesis; espectacular/pretender agents are in other repos)
**Spec:** `openspec/changes/add-git-interface/specs/git/spec.md` — 10 requirements, 19 scenarios. Read it first; it is the contract.
**Tasks:** `openspec/changes/add-git-interface/tasks.md` §1 (1.1–1.11) + §2.2/2.3. §3 (downstream migrations) and §2.1 (spec deployment at archive) are OUT of scope.

## Why

genesis-tpf.1 is the critical path of the add-git-interface epic: all 7 downstream migration tickets (testaruda/espectacular/pretender/dont/wai/whisper/dulce) are blocked on it. It is a well-bounded, single-module TDD implementation — ideal subagent work — while the lead orchestrator runs the epic loop, prepares downstream worktrees, and verifies. Inline implementation in the lead session would violate the orchestrator canon and serialize the epic.

## Desired outcome

A new `genesis::git` module (subprocess-only, read-only) with the exact API surface below, implemented strictly red→green per the tasks file, with contract coverage so `ah check --changes add-git-interface` is green.

```
genesis::git::{
    GitError,          // NotInRepo{start}, Git{args,code,stderr}, Spawn{source}, Io{...}
    GitOutput,         // code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>
    EnvPolicy,         // Inherit (default), StripHookContext
    run(root, policy, args),
    repo_root(), repo_root_from(&Path),
    changed_files(root), changed_files_between(root, base, head),
    uncommitted_files(root), uncommitted_files_lossy(root),
    tracked(root, path), is_ignored(root, path),
    content_hash(root, path), committed_content_hash(root, path),
    parse_porcelain(&str) -> Vec<String>,
}
```

## Out of scope

- `git add`/`commit`/`init`/hook wiring (git_hooks spec owns hooks) — no write-path API.
- Modifying `git_hooks` behavior: `git_hooks::repo_root_from` becomes an alias delegating to `git::repo_root_from` and maps `GitError::NotInRepo` back to `GitHooksError::NotInRepo` (no behavior change; existing git_hooks tests must stay green untouched).
- git2/gitoxide dependencies — subprocess only.
- Spec archive/deployment (openspec/specs/git/spec.md), downstream migrations, closing the bd ticket, pushing (lead does these).

## Red/green/refactor sequence (tasks.md §1)

- 1.1–1.2 RED→GREEN `tests/git_repo_root.rs`: walk-based root detection — normal repo, worktree (`.git` file), inside `.git` dir, nested subdir, not-in-repo typed error. Assert GIT_DIR independence WITHOUT mutating process env (cargo tests run in parallel threads) — use the child-process pattern: parent test spawns `current_exe() --exact <child_test_name> --nocapture` with `GIT_DIR` poisoned via `.env()`; child calls `repo_root_from` and asserts success (proven pattern from genesis-4mq).
- 1.3–1.4 RED→GREEN `tests/git_runner.rs`: canonical `run()` — `EnvPolicy::Inherit` keeps GIT_DIR; `StripHookContext` removes GIT_DIR/GIT_INDEX_FILE/GIT_WORK_TREE. Probe env inheritance per-variable with real git (no PATH shims): GIT_DIR via `rev-parse --git-dir`, GIT_WORK_TREE via `rev-parse --show-toplevel` with a fake worktree, GIT_INDEX_FILE via poisoned corrupt-index child vs stripped success. Never mutate the test process's own env — poison via spawned children only. Non-zero exit → `GitError::Git{args,code,stderr}`; spawn failure (empty-PATH child probe) → `GitError::Spawn`.
- 1.5–1.6 RED→GREEN `tests/git_changed_files.rs`: staged-only vs HEAD, between two revs, unborn HEAD (fresh repo → untracked fallback via `ls-files --others --exclude-standard`, NOT empty set), lossy variant returns empty set on failure; uncommitted includes untracked, changed_files excludes untracked while HEAD resolves.
- 1.7–1.8 RED→GREEN `tests/git_porcelain.rs`: porcelain v1 parse — `XY path`, rename `R  old -> new` (new path only; never the joined `old -> new` string), copy `C`, quoted paths (`core.quotePath` — unquote C-style escapes incl. `\"`, `\\`, `\t`, `\n`, octal `\NNN`), non-ASCII names, empty body → empty result.
- 1.9–1.10 RED→GREEN `tests/git_tracked.rs`: `tracked` via `ls-files --error-unmatch` (exit 0=tracked, 1=untracked, else error); `is_ignored` tri-state single-path via `check-ignore -q` (0=ignored, 1=not-ignored, ≥128 or spawn=failure); `content_hash` via `hash-object` (deterministic; equals `git hash-object` output) and `committed_content_hash` via `rev-parse HEAD:<path>`.
- 1.11 `lib.rs`: `pub mod git;` + doc-header entry in the module list comment.
- §2.2/2.3: contract TOMLs already exist under `.espectacular/changes/add-git-interface/git/` — ensure `ah check --changes add-git-interface` is green (each scenario must map to a real passing test; wire `[tests] cargo = [{ flags = "..." }]` entries to the new test flags if the mapping needs adjusting).
- Tidy: mark tasks.md §1/§2 checkboxes `[x]` as completed.

## Exact files

- `src/git.rs` (new), `src/lib.rs` (module decl + doc list), `src/git_hooks.rs` (alias only)
- `tests/git_repo_root.rs`, `tests/git_runner.rs`, `tests/git_changed_files.rs`, `tests/git_porcelain.rs`, `tests/git_tracked.rs` (new)
- `openspec/changes/add-git-interface/tasks.md` (checkboxes)
- `.espectacular/changes/add-git-interface/git/*.toml` (only if scenario→test flags need wiring)

## Exact commands (completion criteria — all must exit 0)

```bash
cargo test --quiet                       # full suite green (baseline: 574 passed)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
ah check --changes add-git-interface
openspec validate add-git-interface --strict
```

## Conventions & gotchas

- TDD strictly: RED test first (see it fail), then GREEN. No process-env mutation in tests (parallel threads) — use child-process `.env()` poisoning.
- All git subprocesses: `.current_dir(root)` with paths relative to root; use existing genesis patterns (see `src/git_hooks.rs`, `src/update_check.rs` for style).
- Pre-commit hooks (lefthook: `ah check` + fmt/clippy + testaruda select) run on every commit; `bd` operations and `git add -f .beads/issues.jsonl` are NOT yours.
- Repo-root walk must treat a `.git` FILE (worktree link) as a valid root and succeed inside `.git` dirs; reuse the walk from `git_hooks::repo_root_from` (move logic into git.rs, delegate back).

## Spawn model

- Model: `openrouter/z-ai/glm-5.3-flash` (only ready provider; single-family — note for verifier).
- Context ceiling: ≤ 60% of the model window. If you exceed it, commit completed work, write a handoff note in the final report, and stop.

## Follow-up threshold

If a spec scenario cannot be satisfied as written, or the work clearly exceeds one session: STOP, leave tasks.md annotated with what's blocked and why, commit what's green, and report — do not silently reinterpret the spec.
