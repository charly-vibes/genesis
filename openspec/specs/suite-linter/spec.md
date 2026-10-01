# suite-linter Specification

## Purpose
TBD - created by archiving change add-artifact-provenance. Update Purpose after archive.
## Requirements
### Requirement: Managed-block drift check

The suite linter SHALL provide a `ManagedBlockDrift` check that, for each
registered managed block in a repository, regenerates the expected content
and compares it against the file on disk — via the footer content hash where
a footer exists, and via full-text comparison otherwise. Findings SHALL
report the affected file and block, and SHALL carry a caller-supplied fix
command.

#### Scenario: drifted block is reported with a fix command

- **WHEN** a repo file contains a managed block whose content differs from
  the regenerated expected content
- **THEN** the check SHALL produce a `LintResult` with warning severity
  naming the file and block
- **AND** the result SHALL carry the caller-supplied fix command

#### Scenario: current block produces no finding

- **WHEN** every registered block on disk matches its regenerated content
- **THEN** the check SHALL return no findings

#### Scenario: hand-edited block is a finding, not a crash

- **WHEN** a managed block was hand-edited so markers are intact but content
  drifted
- **THEN** the check SHALL report the drift as a finding

