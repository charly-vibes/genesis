# managed-block Specification

## Purpose
TBD - created by archiving change add-artifact-provenance. Update Purpose after archive.
## Requirements
### Requirement: Provenance footer on injected managed blocks

Managed-block injection SHALL support an opt-in provenance footer containing
the generator version, a hash of the block content, and the registry block
name. Blocks injected without the footer SHALL be byte-identical to
pre-footer output.

#### Scenario: footer round-trips through inject and re-inject

- **WHEN** a block is injected with the provenance footer enabled and then
  re-injected (updated in place)
- **THEN** the updated block SHALL carry a footer whose content hash matches
  the newly injected content
- **AND** the begin/end markers SHALL remain recognized by `has_block`

#### Scenario: footer-less injection is unchanged

- **WHEN** an injector built without the footer option injects a block
- **THEN** the written block SHALL be byte-identical to output from the
  pre-footer injector

#### Scenario: content hash excludes the footer line itself

- **WHEN** the content hash is computed
- **THEN** the hash SHALL cover the block content between the markers,
  excluding the footer line itself
- **AND** changing the footer's generator version SHALL NOT change the hash
  for identical content

