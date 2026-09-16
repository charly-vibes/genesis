# envelope delta

## ADDED Requirements

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
