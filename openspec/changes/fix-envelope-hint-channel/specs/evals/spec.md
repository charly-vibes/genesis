# evals delta

## ADDED Requirements

### Requirement: Dual-channel hint resolution

`parse_envelope` SHALL resolve the hint commands of an error envelope from
both the `data.remediation` channel and the top-level `hints` channel, and
SHALL return the union in a stable order (error channel first). A hint entry
MAY be an object carrying `command`, or a bare string naming a command.

#### Scenario: remediation in the error channel is resolved

- **WHEN** `parse_envelope` reads an error envelope whose `data.remediation`
  names one command
- **THEN** the returned outcome SHALL contain that command among its hint
  commands

#### Scenario: top-level hints are resolved

- **WHEN** `parse_envelope` reads an error envelope whose top-level `hints`
  array names one command
- **THEN** the returned outcome SHALL contain that command among its hint
  commands

#### Scenario: both channels contribute

- **WHEN** an error envelope carries a command in `data.remediation` and
  another in the top-level `hints`
- **THEN** the returned outcome SHALL contain both commands
- **AND** the error-channel command SHALL come first

#### Scenario: string-valued hint entries are tolerated

- **WHEN** a hint entry is a bare string naming a command
- **THEN** `parse_envelope` SHALL treat it as that command
- **AND** SHALL NOT fail

#### Scenario: conflicting channels keep both commands

- **WHEN** an error envelope names one command in `data.remediation` and a
      *different* command in the top-level `hints`
- **THEN** the returned outcome SHALL contain both commands
- **AND** SHALL NOT discard either channel's command

#### Scenario: an envelope with no hint channel resolves to no commands

- **WHEN** an error envelope carries neither remediation nor hints
- **THEN** the returned outcome SHALL contain zero hint commands
- **AND** SHALL NOT report an error

### Requirement: Eval fixtures use genesis-built envelopes

Scenario fixtures that stand in for a tool's output SHALL be produced by the
genesis envelope constructors, not hand-written JSON literals. A fixture built
with `Envelope::error` and a remediation entry SHALL resolve to a non-empty
hint-command list.

#### Scenario: a genesis-built error envelope resolves its remediation

- **WHEN** a fixture is built with `Envelope::error` carrying an
  `ErrorResult` with one remediation entry
- **THEN** `parse_envelope` SHALL resolve that entry's command

#### Scenario: a genesis-built success envelope parses as ok

- **WHEN** a fixture is built with `Envelope::success`
- **THEN** `parse_envelope` SHALL report the `Ok` outcome

### Requirement: Hint-following checks are validated against the real shape

The hint-following checks SHALL be exercised against envelopes produced by
the genesis envelope constructors, so that a divergence between the writer
shape and the reader shape fails the suite.

#### Scenario: hint-following scenario passes against a real error envelope

- **WHEN** the hint-following scenario replays a step whose stdout is a
  genesis-built error envelope naming a fix command
- **THEN** the hint check SHALL pass
- **AND** the report SHALL record no failure for that check

#### Scenario: a writer/reader divergence fails the suite

- **WHEN** the envelope constructors stop emitting a channel that
  `parse_envelope` reads
- **THEN** the hint-following scenario SHALL fail
