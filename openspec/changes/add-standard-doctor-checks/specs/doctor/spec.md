# doctor delta

## Purpose

The diagnostic framework the fleet's `doctor` commands are built on: a check
trait, a runner that collects results, and a shared battery of parameterized
checks whose result distinguishes "checked and fine" from "not applicable".

## ADDED Requirements

### Requirement: Shared standard check battery

The framework SHALL provide a standard battery of checks that a tool
configures with its own paths and block names: a config-file check, a
Git-repository check, a managed-block drift check, and a CI-workflow check.

#### Scenario: the battery enumerates the four standard checks

- **WHEN** the standard battery is constructed
- **THEN** it SHALL contain a config-file check, a Git-repository check, a
      managed-block drift check, and a CI-workflow check

#### Scenario: the config check validates the configured file

- **WHEN** the config check runs with a configured config path that exists and
      parses
- **THEN** it SHALL report the config as valid
- **AND** SHALL NOT report a failure for it

#### Scenario: the Git check detects a working directory outside a repository

- **WHEN** the Git-repository check runs in a directory that is not inside a
      Git repository
- **THEN** it SHALL report a failure naming that condition

#### Scenario: the managed-block check detects drift

- **WHEN** the managed-block check runs with a configured block name whose
      injected content differs from the current content
- **THEN** it SHALL report a failure naming the drifted block

#### Scenario: the CI-workflow check detects a missing file

- **WHEN** the CI-workflow check runs with a configured workflow path that
      does not exist
- **THEN** it SHALL report a failure naming the missing workflow

### Requirement: Unconfigured checks report not-applicable

A check the tool did not configure SHALL report a not-applicable result. It
SHALL NOT report a healthy result, so that a report never claims health for
something that was not examined.

#### Scenario: an unconfigured check is not applicable

- **WHEN** the battery runs for a tool that configured no config path
- **THEN** the config check SHALL report not-applicable
- **AND** SHALL NOT report the config as valid

#### Scenario: the report distinguishes applicable from not-applicable

- **WHEN** a report contains both an applicable check and an unconfigured
      check
- **THEN** the report SHALL show a distinct not-applicable state for the
      unconfigured one
- **AND** a reader SHALL be able to tell them apart

### Requirement: Battery results flow into the shared status dashboard

The results of the battery SHALL be usable by the shared status dashboard, so
that a tool's status view reflects the same checks as its doctor command.

#### Scenario: a doctor report contributes to the status dashboard

- **WHEN** a tool registers its doctor runner with the shared status builder
- **THEN** the status view SHALL include the doctor's findings
- **AND** SHALL use the same severity classification as the doctor report
