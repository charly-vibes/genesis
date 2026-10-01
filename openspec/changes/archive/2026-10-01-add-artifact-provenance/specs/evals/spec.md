# evals Specification (delta)

## ADDED Requirements

### Requirement: Receipt terminal-outcome check

A composable `ScenarioCheck` SHALL exist that evaluates a run's final
envelope: it passes when the envelope carries `receipt.terminal_outcome`.
The check itself does not know which runs are mutating — the caller scopes
it (e.g., applies it only to steps it has marked as mutating), mirroring
the envelope contract where tools decide which commands qualify as
mutating.

#### Scenario: envelope with receipt passes the check

- **WHEN** the replayed step's final envelope contains a receipt with a
  terminal outcome
- **THEN** the check SHALL return a passing `CheckOutcome`

#### Scenario: missing receipt is a tool fault

- **WHEN** the caller applies the check to a step it has marked as mutating
  and the replayed step's final envelope carries no receipt
- **THEN** the check SHALL return `tool_fault` citing the missing receipt

#### Scenario: success receipt over failing envelope is a lie

- **WHEN** the final envelope has `ok: false` but its receipt records
  `terminal_outcome: success`
- **THEN** the check SHALL return `tool_fault` with a reason naming the
  contradiction
