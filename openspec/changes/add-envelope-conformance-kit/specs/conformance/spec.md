# conformance delta

## Purpose

A reusable harness that lets any fleet tool assert the shared output contract
— envelope shape, error channel, hint channel, exit-code mapping — from its own
test suite, in one call.

## ADDED Requirements

### Requirement: Envelope shape conformance

The conformance kit SHALL assert that a captured output carries the shared
envelope shape: a boolean `ok`, a non-empty `envelope_version`, a non-empty
`cli_version`, a non-empty `envelope_kind`, and a `data` member.

#### Scenario: a conforming envelope passes the shape check

- **WHEN** the kit inspects the output of a command that printed a
      genesis-shaped envelope
- **THEN** the shape check SHALL pass
- **AND** the kit SHALL report no failure for it

#### Scenario: an envelope missing a member fails the shape check

- **WHEN** the kit inspects output whose envelope lacks the `envelope_kind`
      member
- **THEN** the shape check SHALL fail
- **AND** the failure SHALL name the missing member

### Requirement: Error channel conformance

For an output whose envelope reports failure, the kit SHALL assert that the
error carries at least one remediation entry with a non-empty command.

#### Scenario: a conforming error envelope passes the error check

- **WHEN** the kit inspects an error envelope whose `data.remediation` names a
      command
- **THEN** the error check SHALL pass

#### Scenario: an error envelope without remediation fails the error check

- **WHEN** the kit inspects an error envelope whose `data.remediation` is
      absent or empty
- **THEN** the error check SHALL fail
- **AND** the failure SHALL say the error carries no remediation

### Requirement: Hint channel conformance

For an output whose envelope reports failure, the kit SHALL assert that the
error's suggested command is resolvable through the shared envelope reader —
the same reader the fleet's evaluations use.

#### Scenario: a resolvable remediation passes the hint check

- **WHEN** the kit resolves the hint commands of an error envelope through the
      shared reader
- **THEN** the hint check SHALL pass
- **AND** the resolved list SHALL be non-empty

#### Scenario: an unresolvable remediation fails the hint check

- **WHEN** an error envelope names no command in any channel
- **THEN** the hint check SHALL fail

### Requirement: Exit-code mapping conformance

The kit SHALL assert that an output reporting success exits with status zero,
and an output reporting failure exits with a non-zero status.

#### Scenario: a success exit-code matches the envelope

- **WHEN** a command printed an envelope reporting success and exited zero
- **THEN** the exit-code check SHALL pass

#### Scenario: a failure exit-code matches the envelope

- **WHEN** a command printed an envelope reporting failure and exited
      non-zero
- **THEN** the exit-code check SHALL pass

#### Scenario: a mismatched exit-code fails the check

- **WHEN** a command printed an envelope reporting failure and exited zero
- **THEN** the exit-code check SHALL fail

### Requirement: One-call use from a tool's own suite

A tool SHALL be able to assert the whole contract in one call, given the
captured output of one command invocation.

#### Scenario: a single call reports every violated property

- **WHEN** a tool's captured output violates the shape, error, hint and
      exit-code properties at once
- **THEN** one kit invocation SHALL report each violated property
- **AND** SHALL NOT stop at the first failure
