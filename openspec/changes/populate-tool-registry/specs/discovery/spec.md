# discovery delta

## Purpose

The fleet registry: the manifest that records which suite tools exist, how to
detect each one, and whether each one is installed in the working copy at
hand.

## ADDED Requirements

### Requirement: Declared and installed are distinct

The registry SHALL distinguish a tool that is **declared** (it exists in the
suite) from a tool that is **installed** (it is present in the working copy
being inspected). A discovery result SHALL report both states.

#### Scenario: a declared but absent tool is reported as not installed

- **WHEN** discovery runs in a working copy where a declared tool's marker is
      absent
- **THEN** the tool SHALL be reported as declared and not installed
- **AND** discovery SHALL NOT fail

#### Scenario: a present tool is reported as installed

- **WHEN** discovery runs in a working copy where a declared tool's marker is
      present
- **THEN** the tool SHALL be reported as declared and installed

### Requirement: Detector kinds

A registry entry SHALL name a detector. The supported detector kinds SHALL be:
a file marker at a path, a directory marker at a path, and a command
resolvable on `PATH`.

#### Scenario: a file detector resolves on a present file

- **WHEN** a file-detector entry is checked and the named file exists
- **THEN** the tool SHALL be reported as installed

#### Scenario: a directory detector resolves on a present directory

- **WHEN** a directory-detector entry is checked and the named directory
      exists
- **THEN** the tool SHALL be reported as installed

#### Scenario: a command detector resolves on the executable path

- **WHEN** a command-detector entry is checked and the named executable is
      resolvable
- **THEN** the tool SHALL be reported as installed

#### Scenario: an unknown detector kind is rejected

- **WHEN** a registry entry names a detector kind that is not supported
- **THEN** discovery SHALL report that entry as invalid
- **AND** SHALL NOT treat it as installed

#### Scenario: an entry without a detector is invalid

- **WHEN** a registry entry carries a description but no detector path
- **THEN** discovery SHALL report that entry as invalid
- **AND** SHALL NOT report it as merely absent

#### Scenario: the resolved command path is reported

- **WHEN** a command detector resolves to an executable
- **THEN** discovery SHALL report the path it resolved to
- **AND** a reader SHALL be able to see which binary was matched

### Requirement: Absence is a finding, not a failure

Discovery SHALL NOT return an error when a declared tool is absent or when a
registry entry is invalid. It SHALL report the condition as part of its
result.

#### Scenario: an absent tool yields a report, not an error

- **WHEN** discovery inspects a working copy missing a declared tool
- **THEN** discovery SHALL return a result naming the missing tool
- **AND** SHALL NOT return an error

#### Scenario: a malformed manifest yields a report, not an error

- **WHEN** the manifest cannot be parsed
- **THEN** discovery SHALL return a result reporting the parse failure
- **AND** SHALL NOT panic

### Requirement: Fleet registry content

The registry SHALL be able to describe every Rust CLI in the suite, with a
human-readable description and a detector for each.

#### Scenario: the fleet registry lists the suite's CLIs

- **WHEN** the fleet registry is read
- **THEN** it SHALL contain an entry for each Rust CLI in the suite
- **AND** each entry SHALL carry a description and a detector

#### Scenario: an entry without a detector is invalid

- **WHEN** a registry entry carries a description but no detector
- **THEN** discovery SHALL report that entry as invalid
