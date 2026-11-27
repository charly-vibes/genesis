# envelope delta

## ADDED Requirements

### Requirement: Error remediation channel

Error envelopes SHALL carry their remediation inside `data`, as the
`remediation` array of the error result. Each entry SHALL name the suggested
`command` and a `description`. The top-level `hints` array SHALL NOT be
required on the error path.

#### Scenario: error envelope carries remediation in data

- **WHEN** a tool builds an error envelope with one remediation entry
- **THEN** the serialized envelope SHALL contain that entry under
  `data.remediation`
- **AND** the entry SHALL carry a non-empty `command` and `description`

#### Scenario: error envelope does not require top-level hints

- **WHEN** a tool builds an error envelope
- **THEN** the serialized envelope SHALL NOT require a `hints` array for a
  reader to find the remediation

### Requirement: Success-path hint channel

The top-level `hints` array SHALL remain the channel for next-step suggestions
attached to a successful command. Its entries SHALL be objects carrying
`command` and `description`.

#### Scenario: success envelope carries next-step hints

- **WHEN** a tool builds a successful envelope with two hints
- **THEN** the serialized envelope SHALL contain both entries in the
  top-level `hints` array
- **AND** each entry SHALL carry a `command` and a `description`

### Requirement: Reader tolerance for the deprecated error-path hint channel

A reader of an error envelope MAY additionally accept hint entries in the
top-level `hints` array, as emitted by tools that predate the error
remediation channel. This tolerance is read-only: no writer SHALL be required
to emit top-level hints on the error path.

#### Scenario: legacy top-level hints remain readable

- **WHEN** a reader encounters an error envelope that also carries hint
  entries in the top-level `hints` array
- **THEN** the reader SHALL accept the envelope
- **AND** SHALL NOT report the envelope as malformed
