## MODIFIED Requirements

### Requirement: Lefthook managed-block wiring

genesis SHALL provide `lefthook::ensure_wired()` that injects a
`managed_block`-formatted entry into the `commands:` mapping of the
`pre-commit:` or `pre-push:` section of a lefthook config, idempotently.
Caller content SHALL be the entries belonging inside `commands:` (inserted
verbatim, no re-indentation).

#### Scenario: block nested under existing commands mapping

- **WHEN** `lefthook.yml` contains a `pre-commit:` section whose body has a
  `commands:` key
- **AND** `ensure_wired(root, Stage::PreCommit, block, content)` is called
  with content written for the `commands:` child level
- **THEN** the managed block SHALL be inserted directly after the
  `commands:` line, with no re-indentation of the content
- **AND** the rest of the file SHALL be unchanged
- **AND** the stage SHALL NOT gain a second `commands:` key

#### Scenario: commands mapping created when the stage lacks one

- **WHEN** `lefthook.yml` contains a `pre-commit:` section whose body has
  no `commands:` key
- **AND** `ensure_wired()` is called
- **THEN** a `  commands:` line SHALL be emitted directly after the stage
  anchor, followed by the managed block

#### Scenario: section created when missing

- **WHEN** `lefthook.yml` exists but has no `pre-push:` section
- **AND** `ensure_wired(root, Stage::PrePush, block, content)` is called
- **THEN** a `pre-push:` section SHALL be created containing a
  `  commands:` line followed by the managed block

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
