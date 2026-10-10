# git delta

## ADDED Requirements

### Requirement: Read-only plumbing only

The `git` module SHALL expose read-only repository queries (repo root,
changed/uncommitted files, tracked/ignored state, content hash) and SHALL
NOT expose mutating operations (`init`, `add`, `commit`) or hook wiring
(owned by the `git-hooks` spec).

#### Scenario: no write-path API

- **WHEN** the module's public API is enumerated
- **THEN** no function SHALL spawn `git init`, `git add`, or `git commit`

### Requirement: Subprocess-only substrate

The module SHALL run the `git` binary via subprocess. It SHALL NOT link
`git2`, `gitoxide`, or any other git library.

#### Scenario: no git library in the dependency graph

- **WHEN** `Cargo.toml` dependency resolution is inspected
- **THEN** neither `git2` nor `gitoxide` SHALL appear among genesis's
  regular dependencies

### Requirement: Walk-based repo-root detection

`repo_root` SHALL locate the repository root by walking up from the starting
directory until a `.git` entry (file or directory) is found, SHALL treat a
`.git` file (worktree link) as a valid repository root, SHALL succeed inside
`.git` directories, and SHALL NOT be influenced by `GIT_DIR` environment
state. Out-of-repo starts SHALL yield a typed error.

#### Scenario: worktree with a `.git` file resolves to the worktree root

- **WHEN** the walk starts in a linked-worktree directory whose `.git` is a
  file
- **THEN** `repo_root` SHALL return that worktree directory

#### Scenario: nested subdirectory resolves to the enclosing root

- **WHEN** the walk starts in a subdirectory two levels below a repository
  root
- **THEN** `repo_root` SHALL return the repository root

#### Scenario: outside a repository yields a typed error

- **WHEN** the walk reaches the filesystem root without finding `.git`
- **THEN** `repo_root` SHALL return a `NotInRepo`-style typed error
- **AND** SHALL NOT panic

### Requirement: Canonical runner with opt-in env hygiene

The module SHALL provide one canonical runner for git invocations. By
default the runner SHALL inherit the caller's environment. Stripping
`GIT_DIR`, `GIT_INDEX_FILE`, and `GIT_WORK_TREE` SHALL be opt-in via an
explicit environment policy, because hook-context processes legitimately
inherit those variables.

#### Scenario: default policy inherits git context variables

- **WHEN** the runner is invoked with the default environment policy and
  `GIT_DIR` is set in the process environment
- **THEN** the spawned `git` process SHALL receive `GIT_DIR`

#### Scenario: strip-hook-context policy removes hook-injected variables

- **WHEN** the runner is invoked with the strip-hook-context policy
- **THEN** the spawned `git` process SHALL NOT receive `GIT_DIR`,
  `GIT_INDEX_FILE`, or `GIT_WORK_TREE`

### Requirement: Declared failure semantics

Query operations SHALL return typed errors on failure. Silent degradation
SHALL exist only in explicitly named `*_lossy` variants whose
empty-collection-on-failure behavior is documented. The same operation SHALL
NOT have both silent and erroring behavior hidden behind one signature.

#### Scenario: a failed git invocation surfaces as a typed error

- **WHEN** the underlying `git` process fails (non-zero exit or spawn
  failure) during a non-lossy query
- **THEN** the operation SHALL return a typed error carrying the args and
  exit status
- **AND** SHALL NOT return an empty result

#### Scenario: lossy variants degrade silently by contract

- **WHEN** the underlying `git` process fails during a `*_lossy` variant
- **THEN** the variant SHALL return an empty collection
- **AND** its documentation SHALL state the degradation explicitly

#### Scenario: changed-files lossy variant degrades silently by contract

- **WHEN** the underlying `git` process fails during `changed_files_lossy`
- **THEN** the variant SHALL return an empty collection
- **AND** when `git` succeeds, the variant SHALL match `changed_files`
- **AND** its documentation SHALL state the degradation explicitly

### Requirement: Policy-aware named helpers

Every named helper that spawns git (`changed_files`,
`changed_files_between`, `uncommitted_files`, `tracked`, `is_ignored`,
`content_hash`, `committed_content_hash`, and the single-path status)
SHALL expose a `*_with_policy` variant taking an explicit environment
policy as its last argument. The plain variants SHALL keep the
`Inherit` default, and both variants SHALL share one internal
implementation (no logic duplication).

#### Scenario: policy variants propagate the environment policy

- **WHEN** a named helper's `*_with_policy` variant is invoked with the
  strip-hook-context policy in a hook-like environment (`GIT_DIR` set to
  a value that cannot be a git directory)
- **THEN** the spawned `git` process SHALL NOT receive `GIT_DIR`
- **AND** the helper SHALL succeed with correct values

#### Scenario: named helpers default to inherit

- **WHEN** a plain named helper is invoked in a hook-like environment
  (`GIT_DIR` set to a value that cannot be a git directory)
- **THEN** the spawned `git` process SHALL receive `GIT_DIR`
- **AND** the helper SHALL fail with a typed error (inheritance is
  observable)

### Requirement: Single-path status with porcelain XY

`path_status` SHALL query `git status --porcelain -- <path>` for exactly
one path and SHALL map the `XY` code to a tri-state: `??` → untracked;
unmerged entries (`U` in either column, or `DD`/`AA`) → dirty (documented
mapping); non-blank `Y` (worktree differs from index) → dirty; non-blank
`X` (index differs from `HEAD`) → staged. A clean path (empty porcelain
body) SHALL yield no status. The worktree column SHALL win when both
columns are set. Git failures SHALL surface as typed errors.

#### Scenario: single-path status maps the XY columns

- **WHEN** `path_status` is called for an untracked path, a
  worktree-modified path, and a staged-only path
- **THEN** the results SHALL be untracked, dirty, and staged
  respectively
- **AND** when both columns are set (`MM`), the worktree state SHALL win
  (dirty)

#### Scenario: a clean path reports no status

- **WHEN** `path_status` is called for a committed, unmodified path
- **THEN** the result SHALL be no status (empty porcelain body)
- **AND** the operation SHALL succeed

#### Scenario: unmerged states map to dirty

- **WHEN** the index carries unmerged entries for the path (`UU`)
- **THEN** `path_status` SHALL report dirty
- **AND** SHALL NOT report staged or untracked

#### Scenario: absolute paths under the root are accepted

- **WHEN** `path_status` is called with an absolute path under `root`
- **THEN** the path SHALL be relativized against `root` and the query
  SHALL succeed like its relative form

#### Scenario: a git failure surfaces as a typed error

- **WHEN** `path_status` runs outside a repository
- **THEN** the operation SHALL return a typed error
- **AND** SHALL NOT report any status

### Requirement: Porcelain v1 parsing contract

Uncommitted-file enumeration SHALL parse `git status --porcelain` (v1)
including rename/copy records `XY old -> new` (reporting `new`), quoted and
non-ASCII paths, and SHALL handle an empty status body as an empty result.

#### Scenario: rename records report the new path

- **WHEN** the status body contains `R  old-name.rs -> new-name.rs`
- **THEN** the parsed file list SHALL contain `new-name.rs`
- **AND** SHALL NOT contain the joined string `old-name.rs -> new-name.rs`

#### Scenario: quoted paths are unquoted

- **WHEN** the status body contains a quoted path entry (non-ASCII or
  special-character filename)
- **THEN** the parsed file list SHALL contain the unquoted path

#### Scenario: clean working tree parses to empty

- **WHEN** the status body is empty
- **THEN** the parsed file list SHALL be empty
- **AND** the operation SHALL succeed

### Requirement: Unborn-HEAD fallback for changed files

Changed-file enumeration against `HEAD` SHALL detect the unborn-HEAD case
(where `git diff --name-only HEAD` exits 128 because HEAD does not resolve)
and SHALL fall back to enumerating untracked files rather than returning an
empty set.

#### Scenario: fresh repo with no commits reports untracked files

- **WHEN** `changed_files` runs against `HEAD` in a repository with no
  commits and untracked files exist
- **THEN** the result SHALL include those untracked files
- **AND** SHALL NOT be an empty set

### Requirement: Enumeration set definitions

`uncommitted_files` SHALL return every entry of the porcelain status —
staged, unstaged, renamed (new path), and untracked files.
`changed_files` SHALL return tracked changes against the comparison base
(`HEAD`, or the two revisions of `changed_files_between`) and SHALL NOT
include untracked files while `HEAD` resolves. The unborn-HEAD fallback is
the only mode in which `changed_files` reports untracked files.

#### Scenario: uncommitted files include untracked entries

- **WHEN** a repository has both a modified tracked file and an untracked
  file
- **THEN** `uncommitted_files` SHALL include both

#### Scenario: changed files exclude untracked files when HEAD exists

- **WHEN** a repository with commits has both a modified tracked file and
  an untracked file
- **THEN** `changed_files` SHALL include the modified tracked file
- **AND** SHALL NOT include the untracked file

### Requirement: Tracked and content-hash queries

`tracked` SHALL report whether the repository knows a path (via
`git ls-files`), independent of working-tree modifications.
`content_hash` SHALL hash worktree files via `git hash-object` and SHALL
resolve committed content via `git rev-parse HEAD:<path>`; both SHALL be
deterministic for the same content.

#### Scenario: tracked reports files known to git

- **WHEN** `tracked` is called for a committed path and for a path never
  added to the index
- **THEN** the committed path SHALL report tracked
- **AND** the never-added path SHALL report untracked

#### Scenario: content hash is deterministic per content

- **WHEN** `content_hash` is called twice for the same worktree file
- **THEN** both calls SHALL return the same hash
- **AND** the hash SHALL equal `git hash-object` output for that file

### Requirement: check-ignore tri-state, single path

`is_ignored` SHALL accept exactly one path argument and SHALL distinguish
three outcomes: ignored, not ignored, and git failure (error), mapping
git's exit codes 0, 1, and ≥128 respectively. Multi-path invocation SHALL
not be exposed, because `check-ignore` exits 0 when any of several paths is
ignored, which cannot express a per-path tri-state.

#### Scenario: not-ignored is distinct from error

- **WHEN** `check-ignore` exits 1 for a path that is not ignored
- **THEN** the result SHALL report not-ignored
- **AND** SHALL NOT report an error

#### Scenario: a git failure is an error, not "not ignored"

- **WHEN** `check-ignore` fails to spawn or exits with code 128 or higher
- **THEN** the result SHALL report an error
- **AND** SHALL NOT report not-ignored
