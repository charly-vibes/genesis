# aix Specification (delta)

## ADDED Requirements

### Requirement: Provenance footer on generated artifacts

Generated `llms.txt` / `llm.txt` artifacts SHALL support an opt-in provenance
footer line recording the generator version and a content hash. The footer
SHALL be appended after all content sections. Deterministic generators SHALL
remain deterministic: the timestamped footer variant is a separate function.

#### Scenario: default generators stay deterministic

- **WHEN** `generate_llms_txt` or `generate_llms_txt_bounded` is called twice
  with identical inputs
- **THEN** both outputs SHALL be byte-identical and free of timestamps

#### Scenario: footer states generator version and content hash

- **WHEN** an artifact is generated with the provenance footer enabled
- **THEN** the final lines SHALL identify the generating tool version and a
  hash of the artifact content
- **AND** the token-cost estimate of the artifact SHALL include the footer

#### Scenario: bounded generation degrades content but keeps the footer

- **WHEN** a bounded artifact overflows its budget and degrades descriptions
- **THEN** the footer SHALL reflect the hash of the *degraded* content
