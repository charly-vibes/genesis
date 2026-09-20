# evals delta

## MODIFIED Requirements

### Requirement: Model attribution on scenario reports

Scenario reports SHALL optionally record which model replayed the scenario,
recording the raw model id verbatim — including `:free` and any deployment
suffix — because matrix comparisons across time depend on the unnormalized
id. (Repetition indexes, added by the evals-guidelines capability, ride on
the same report.)

#### Scenario: report carries model when provided

- **WHEN** a replay is attributed to a model name
- **THEN** the scenario report SHALL include that name for matrix
  comparison
- **AND** the id SHALL be recorded verbatim, including `:free` or
  deployment suffixes

#### Scenario: report without attribution stays valid

- **WHEN** no model name is provided
- **THEN** the report SHALL serialize without a model field

## ADDED Requirements

### Requirement: Action-format-violation fault code

genesis SHALL extend `ErrorTaxonomy` with `ActionFormatViolation`
(`ERR_ACTION_FORMAT_VIOLATION`): the live agent failed to produce a
parseable structured action after one re-ask. It SHALL be distinct from
`ToolExecutionHallucination`, which remains a replay-tier code for claims
of runs that never happened.

#### Scenario: round-trip through the code

- **WHEN** `from_code("ERR_ACTION_FORMAT_VIOLATION")` is called
- **THEN** it SHALL return the `ActionFormatViolation` variant
- **AND** `code()` on that variant SHALL return the same string

#### Scenario: existing codes unchanged

- **WHEN** any pre-existing `ERR_*` code is round-tripped
- **THEN** the mapping SHALL be unchanged
