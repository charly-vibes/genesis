# aix delta

## ADDED Requirements

### Requirement: Token-cost estimate on generated artifacts

genesis SHALL provide a token-cost estimate for generated llms.txt / llm.txt
artifacts, clearly labeled as a heuristic estimate, without changing the
existing generator function signatures.

#### Scenario: estimate is labeled, not mistaken for a count

- **WHEN** any generated artifact is measured with `estimate_token_cost`
- **THEN** the returned `TokenCost` SHALL include the chars/4 heuristic name
  alongside the estimate

#### Scenario: existing generator signatures are unchanged

- **WHEN** `generate_llms_txt` or `generate_llm_txt` is called
- **THEN** the return type SHALL remain `String`
- **AND** existing callers SHALL compile without modification

#### Scenario: adding modules increases the estimate

- **WHEN** a `ModuleEntry` is added to the project metadata
- **THEN** the reported estimate SHALL be strictly greater than before

### Requirement: Budget-bounded generation with graceful degradation

genesis SHALL provide bounded variants of both generators (modules-based
llms.txt and section-based llm.txt) that produce artifacts under a declared
token budget by degrading content granularity deterministically instead of
overflowing or refusing.

#### Scenario: artifact within budget is generated verbatim

- **WHEN** the full artifact's estimate is at or under the declared budget
- **THEN** the bounded variant SHALL return byte-identical output to the
  unbudgeted generator

#### Scenario: over-budget artifact degrades descriptions first

- **WHEN** the full artifact exceeds the budget
- **THEN** module descriptions SHALL be truncated to their first sentence
  before any section is dropped

#### Scenario: degradation drops optional sections before tables

- **WHEN** degradation must remove content beyond description truncation
- **THEN** raw/optional sections SHALL be dropped before module tables
- **AND** module headings SHALL survive all degradation levels

#### Scenario: degradation is deterministic

- **WHEN** the same inputs and budget are used twice
- **THEN** both outputs SHALL be byte-identical
