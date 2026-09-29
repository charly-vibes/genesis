# fix-hooks-dir-scopes — Spec Delta

## MODIFIED Requirements

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

#### Scenario: install respects core.hooksPath at any scope

- **WHEN** the repository has a `core.hooksPath` config set at the local,
  global, or system scope
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

## ADDED Requirements

### Requirement: Effective hooks directory resolution

genesis SHALL provide `effective_hooks_dir()` that resolves
`core.hooksPath` across all config scopes (local → global → system),
matching git's own precedence, and reports which scope the value came
from so consumers can detect scope mismatches (e.g. a hook installed
locally while git invokes hooks from a global path).

#### Scenario: global scope is honored

- **WHEN** no local `core.hooksPath` is set
- **AND** the user's global git config sets `core.hooksPath`
- **THEN** `effective_hooks_dir(root)` SHALL resolve to the global value
- **AND** `resolve_hooks_dir(root)` SHALL return the same path

#### Scenario: local scope wins over global

- **WHEN** both local and global `core.hooksPath` are set
- **THEN** the resolved directory SHALL be the local value

#### Scenario: system scope is honored when higher scopes are unset

- **WHEN** neither local nor global `core.hooksPath` is set
- **AND** the system git config sets `core.hooksPath`
- **THEN** the resolved directory SHALL be the system value

#### Scenario: scope is reported to consumers

- **WHEN** `effective_hooks_dir(root)` is called
- **THEN** the result SHALL report the scope the value was found in:
  `Local`, `Global`, `System`, `Default` (unset anywhere), or `Disabled`
  (empty-string value)

#### Scenario: empty-string config is surfaced as disabled

- **WHEN** `core.hooksPath` is set to the empty string at the effective
  scope
- **THEN** `effective_hooks_dir(root)` SHALL report `HooksDirScope::Disabled`
- **AND** `resolve_hooks_dir(root)` SHALL keep its documented fallback
  (the default `.git/hooks`) so existing callers are unaffected

#### Scenario: unset config falls back to the default

- **WHEN** no scope sets `core.hooksPath`
- **THEN** `effective_hooks_dir(root)` SHALL report `HooksDirScope::Default`
  with the path `.git/hooks` under the repository root

#### Scenario: relative values resolve against the repository root

- **WHEN** the effective `core.hooksPath` value is a relative path
- **THEN** the resolved directory SHALL be the value joined onto the
  repository root
