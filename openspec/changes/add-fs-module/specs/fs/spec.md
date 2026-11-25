# fs delta

## Purpose

Shared file-writing primitives for the fleet: replacing a file atomically,
reading and writing JSON, and the durability and platform guarantees those
operations carry.

## ADDED Requirements

### Requirement: Atomic replace

`atomic_write` SHALL replace the target file in one observable step. It SHALL
stage the bytes in a uniquely-named sibling temporary file in the target's own
directory, flush them to the device, and rename the temporary file over the
target.

#### Scenario: a reader never observes a partial file

- **WHEN** a writer replaces a file while a reader reads it
- **THEN** the reader SHALL observe either the complete previous content or
  the complete new content
- **AND** SHALL NOT observe a truncated or partially written file

#### Scenario: the prior content survives a failed write

- **WHEN** the write fails before the rename step
- **THEN** the target file SHALL be left byte-for-byte unchanged
- **AND** the temporary file SHALL be removed

#### Scenario: the rename happens on the same filesystem

- **WHEN** the temporary file is staged
- **THEN** it SHALL be created in the target's own directory
- **AND** SHALL NOT be created in a system temporary directory

### Requirement: Unique staging names

The staging file name SHALL be unique per writer, so that two concurrent
writers to the same target never share a staging path.

#### Scenario: concurrent writers use distinct staging files

- **WHEN** two writers stage a replacement for the same target at the same
  time
- **THEN** their staging file names SHALL differ
- **AND** neither writer SHALL overwrite the other's staging file

#### Scenario: repeated writes within one process do not collide

- **WHEN** the same process writes the same target twice in succession
- **THEN** the second staging file SHALL NOT reuse the first staging file name

#### Scenario: a stale staging file is identifiable

- **WHEN** a writer is killed before the rename and leaves a staging file
      behind
- **THEN** the staging file's name SHALL be recognizable as a staging file of
      the target
- **AND** sweeping stale staging files SHALL NOT be required for the write to
      have succeeded

### Requirement: Durability before the rename

The staged bytes SHALL be flushed to the device before the rename step, so that
a crash after the rename cannot leave a renamed-but-empty file.

#### Scenario: staged content is flushed before the rename

- **WHEN** the staging file is closed
- **THEN** the bytes SHALL have been flushed to the device
- **AND** the rename SHALL happen only after that flush

### Requirement: Windows rename resilience

On Windows, the rename step SHALL retry a transient sharing violation, because
`std::fs::rename` there replaces an existing target and can fail while
another process holds the target open.

#### Scenario: a transient sharing violation is retried

- **WHEN** the rename fails with a sharing violation
- **THEN** the operation SHALL retry the rename
- **AND** SHALL report the failure only after the retries are exhausted

#### Scenario: a non-sharing rename failure is not retried

- **WHEN** the rename fails for a reason other than a sharing violation
- **THEN** the operation SHALL report the failure without retrying

#### Scenario: the retry is bounded

- **WHEN** the rename keeps failing with a sharing violation
- **THEN** the operation SHALL stop after a bounded number of attempts, or
      after a bounded total time
- **AND** SHALL report the failure rather than retrying indefinitely

### Requirement: JSON helpers

`read_json` SHALL deserialize a JSON file into a caller-supplied type, and
`atomic_write_json` SHALL serialize a value and write it through the atomic
replace path.

#### Scenario: a JSON file round-trips

- **WHEN** a value is written with `atomic_write_json` and read back with
  `read_json`
- **THEN** the value read back SHALL equal the value written

#### Scenario: malformed JSON yields a typed error

- **WHEN** `read_json` reads a file whose content is not valid JSON
- **THEN** it SHALL return a typed error
- **AND** SHALL NOT panic
