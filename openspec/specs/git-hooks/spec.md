# git-hooks Specification

## Purpose
TBD - created by archiving change add-git-hooks. Update Purpose after archive.
## Requirements
### Requirement: Managed block format compatibility

The `git_hooks` module's lefthook injection SHALL use the existing
`genesis::managed_block` injector and block format, not bespoke string
surgery.

#### Scenario: injected block is recognized by managed_block reader

- **WHEN** `ensure_wired()` injects a block into `lefthook.yml`
- **AND** the file is parsed with `managed_block`'s reader
- **THEN** the block SHALL be found with its tool tag and contents intact

### Requirement: Tool-specific gates stay in tools

The `git_hooks` module SHALL contain only git-hook mechanics. No tool
gate command (e.g. `ah check`, `pretender check`, `testaruda select`,
`just check-claims`) SHALL appear in the module.

#### Scenario: module has no tool-specific defaults

- **WHEN** the `git_hooks` module source is inspected
- **THEN** it SHALL contain no hardcoded gate command strings from
  consuming tools
- **AND** callers SHALL pass their marker and block contents as parameters

### Requirement: Repository root discovery

genesis SHALL provide `repo_root()` that locates the enclosing git
repository by walking parent directories until a `.git` entry is found.

#### Scenario: root found from a subdirectory

- **WHEN** `repo_root()` is called from a directory nested inside a git repository
- **THEN** it SHALL return the path of the directory containing `.git`

#### Scenario: outside a repository fails

- **WHEN** `repo_root()` is called from a directory with no `.git` entry
  in any parent
- **THEN** it SHALL return an error naming the problem

### Requirement: Marker-based hook installation

genesis SHALL provide `install()` that writes an executable hook script to
the repository's hook directory, identified by an ownership marker comment.

#### Scenario: install writes an executable hook

- **WHEN** `install(root, HookName::PreCommit, marker, script)` is called
- **THEN** `<hooks>/pre-commit` SHALL contain the marker and script
- **AND** the file SHALL have executable permissions (0755)

#### Scenario: hooks directory is created on demand

- **WHEN** `install()` is called and `<hooks>/` does not exist
- **THEN** the directory SHALL be created before the hook is written

#### Scenario: install respects core.hooksPath

- **WHEN** the repository has a local `core.hooksPath` config set
- **AND** `install()` is called
- **THEN** the hook SHALL be written into the resolved hooks directory
  instead of `.git/hooks/`

#### Scenario: install refuses a foreign hook

- **WHEN** `install()` is called on a hook file that exists
- **AND** the file does not contain the ownership marker
- **THEN** an error SHALL be returned
- **AND** the existing hook SHALL NOT be modified

#### Scenario: install is idempotent for own hooks

- **WHEN** `install()` is called on a hook file that already contains the
  ownership marker
- **THEN** the file SHALL be overwritten with the new contents
- **AND** no error SHALL be returned

### Requirement: Marker-based hook uninstall

genesis SHALL provide `uninstall()` that removes a hook only when the
ownership marker matches.

#### Scenario: uninstall removes an owned hook

- **WHEN** `uninstall(root, hook_name, marker)` is called
- **AND** the hook file exists and contains the marker
- **THEN** the hook file SHALL be removed

#### Scenario: uninstall is a no-op for missing hooks

- **WHEN** `uninstall()` is called and the hook file does not exist
- **THEN** it SHALL return success without error

#### Scenario: uninstall refuses a foreign hook

- **WHEN** `uninstall()` is called on a hook file without the marker
- **THEN** an error SHALL be returned
- **AND** the hook file SHALL NOT be removed

### Requirement: Hook owner detection

genesis SHALL provide `owner()` that identifies which known tool owns a
hook file, using the same sigil conventions wai's `way/hooks.rs` uses.

#### Scenario: lefthook-owned hook is detected

- **WHEN** a `pre-commit` hook contains the string `lefthook`
- **AND** `owner(root, HookName::PreCommit)` is called
- **THEN** it SHALL return `Owner::Lefthook`

#### Scenario: more-specific sigils win over generic ones

- **WHEN** a hook contains both a generic and a more-specific sigil
- **THEN** `owner()` SHALL return the more-specific owner

#### Scenario: unknown or missing hook has no owner

- **WHEN** the hook file is missing or contains no known sigil
- **THEN** `owner()` SHALL return `None`

### Requirement: Hook framework detection

genesis SHALL provide `framework()` that detects which hook-management
framework a repository uses, superseding espectacular's
`detect_hook_framework()` with wai's delegation handling. A repository is
assumed to use at most one framework; `framework()` SHALL report that one
framework generically (including husky, which has no root config and is
detected via hook-file sigil).

#### Scenario: lefthook config present

- **WHEN** the repository root contains `lefthook.yml` or `lefthook.yaml`
- **AND** `framework(root)` is called
- **THEN** it SHALL return `Framework::Lefthook`

#### Scenario: prek config present

- **WHEN** no lefthook config exists and the root contains `prek.toml`
- **THEN** `framework()` SHALL return `Framework::Prek`

#### Scenario: husky-owned hook without root config

- **WHEN** no lefthook or prek config exists in the repository root
- **AND** the hook file contains the husky sigil
- **THEN** `framework()` SHALL return `Framework::Husky`

#### Scenario: framework signals coexist

- **WHEN** more than one framework signal exists (a root config and/or
  hook-file sigils) — expected to be rare since a repository is assumed
  to use one framework
- **THEN** `framework()` SHALL report deterministically by precedence:
  `Lefthook`, then `Prek`, then `Husky`

#### Scenario: no known framework

- **WHEN** no known framework config exists in the repository
- **THEN** `framework()` SHALL return `Framework::None`

### Requirement: Lefthook managed-block wiring

genesis SHALL provide `lefthook::ensure_wired()` that injects a
`managed_block`-formatted entry into the `pre-commit:` or `pre-push:`
section of a lefthook config, idempotently.

#### Scenario: block injected under existing pre-commit

- **WHEN** `lefthook.yml` contains a `pre-commit:` section
- **AND** `ensure_wired(root, Stage::PreCommit, block)` is called
- **THEN** the managed block SHALL be inserted directly after
  `pre-commit:`
- **AND** the rest of the file SHALL be unchanged

#### Scenario: section created when missing

- **WHEN** `lefthook.yml` exists but has no `pre-push:` section
- **AND** `ensure_wired(root, Stage::PrePush, block)` is called
- **THEN** a `pre-push:` section SHALL be created containing the managed block

#### Scenario: wiring is idempotent

- **WHEN** `ensure_wired()` is called and the config already contains the
  block's content
- **THEN** the file SHALL be unchanged and success returned

#### Scenario: missing config is reported, not created

- **WHEN** no `lefthook.yml` or `lefthook.yaml` exists in the repository root
- **THEN** `ensure_wired()` SHALL return an error telling the caller no
  supported hook framework config was found

#### Scenario: unanchorable config errors without modification

- **WHEN** the lefthook config cannot be anchored (e.g. the stage key is
  quoted, as in `"pre-push":`, or the structure is unrecognized)
- **AND** `ensure_wired()` is called
- **THEN** an error SHALL be returned describing the anchor failure
- **AND** the config file SHALL NOT be modified

### Requirement: Wiring verification

genesis SHALL provide `lefthook::is_wired()` that reports whether a given
gate command appears in a stage's section of the lefthook config, for use
by doctor checks (wai's lefthook-wiring check, espectacular's doctor).

#### Scenario: gate command present in stage

- **WHEN** `lefthook.yml`'s `pre-commit:` section contains `run: ah check`
- **AND** `is_wired(root, Stage::PreCommit, "ah check")` is called
- **THEN** it SHALL return `true`

#### Scenario: gate command absent from stage

- **WHEN** the stage's section does not contain the command
- **OR** the stage section does not exist
- **OR** no lefthook config exists
- **THEN** `is_wired()` SHALL return `false`

#### Scenario: command in the wrong stage is not wired

- **WHEN** the command appears only under `pre-push:`
- **AND** `is_wired(root, Stage::PreCommit, command)` is called
- **THEN** it SHALL return `false`

