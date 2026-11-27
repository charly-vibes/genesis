# envelope Specification

## Purpose
TBD - created by archiving change add-aix-eval-loop. Update Purpose after archive.
## Requirements
### Requirement: Receipt metadata on envelopes

genesis SHALL allow tools to attach optional receipt metadata to an envelope
recording the terminal outcome, retry identity, and user-visible evidence of a
command run.

#### Scenario: envelope without receipt serializes unchanged

- **WHEN** an envelope is constructed without `with_receipt()`
- **THEN** the serialized JSON SHALL NOT contain a `receipt` key
- **AND** `parse_envelope` SHALL accept it exactly as before this change

#### Scenario: receipt round-trips

- **WHEN** an envelope carries `ReceiptMeta` with terminal outcome, attempt,
  and evidence
- **THEN** serde round-trip SHALL preserve all fields
- **AND** `parse_envelope` SHALL accept the enriched envelope without change
  to its return type

### Requirement: Terminal outcome classification

`ReceiptMeta` SHALL classify the terminal outcome of a run as Success, Failure,
Timeout, or Cancelled.

#### Scenario: every terminal boundary is representable

- **WHEN** a tool records a run that ended in success, failure, timeout, or
  cancellation
- **THEN** each case SHALL map to exactly one `TerminalOutcome` variant

#### Scenario: timeout is explicit, not silent failure

- **WHEN** a tool's command hits its deadline
- **THEN** the tool SHALL be able to emit `TerminalOutcome::Timeout` in the
  receipt
- **AND** the envelope's `ok` field and the receipt's outcome SHALL remain
  independently settable (a delivered result with a timed-out follow-up is
  representable)

### Requirement: Retry identity for mutating commands

`ReceiptMeta` SHALL support an attempt counter and optional idempotency key so
that duplicate or retried runs are distinguishable.

#### Scenario: retried command carries attempt number

- **WHEN** a tool emits envelopes for attempt 1 and attempt 3 of the same
  mutating command
- **THEN** the receipts SHALL show `attempt` 1 and 3 respectively
- **AND** both MAY share the same idempotency key


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
