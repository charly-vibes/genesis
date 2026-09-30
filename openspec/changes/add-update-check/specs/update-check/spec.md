# update-check delta

## ADDED Requirements

### Requirement: Update notifications serve binaries, not libraries

The `update_check` module SHALL check crates.io for a newer version of the
CALLER'S own crate, identified by a caller-supplied crate name and installed
version. genesis SHALL NOT notify about its own version from inside a
dependent, and the HTTP stack SHALL be pulled only by dependents that opt in
to the `update-check` feature.

#### Scenario: a binary wires its own crate name

- **WHEN** a binary calls `check("wai", "2026.9.28")`
- **THEN** the module SHALL query the crates.io API for `wai` only
- **AND** any surfaced notice SHALL name `wai` and include the fix command
  `cargo install wai`

#### Scenario: the HTTP stack stays optional

- **WHEN** the crate is inspected with the `update-check` feature absent
- **THEN** `ureq` SHALL remain an optional dependency selected only by the
  `update-check` feature
- **AND** the module SHALL NOT be compiled into dependents that do not opt in

### Requirement: Cached-passive checking

The check SHALL perform at most one HTTP fetch per cache TTL (default 7
days) per crate, persisting the latest known version and check timestamp.
A fresh cache SHALL serve results with zero HTTP calls and zero added
latency.

#### Scenario: a fresh cache surfaces the cached update without any HTTP

- **WHEN** the cache for the crate is younger than the TTL
- **AND** the cached latest version differs from the installed version
- **THEN** the check SHALL return the cached update info
- **AND** SHALL NOT perform any HTTP request

#### Scenario: a fresh cache with no update returns none without any HTTP

- **WHEN** the cache for the crate is younger than the TTL
- **AND** the cached latest version equals the installed version
- **THEN** the check SHALL return none
- **AND** SHALL NOT perform any HTTP request

#### Scenario: a stale cache triggers exactly one fetch and rewrites the cache

- **WHEN** the cache for the crate is older than the TTL
- **AND** the crates.io API responds with the latest stable version
- **THEN** the check SHALL return the update info
- **AND** SHALL rewrite the cache with the new version and check timestamp

#### Scenario: a missing cache fetches on first run

- **WHEN** no cache file exists for the crate
- **AND** the crates.io API responds
- **THEN** the check SHALL return the update info when a newer stable
  version exists
- **AND** SHALL write the cache for subsequent runs

### Requirement: Fail-silent robustness

Every error path — HTTP errors, unreachable servers, corrupt cache files,
unwritable cache directories — SHALL degrade to no update surfaced. The
check SHALL NOT panic and SHALL NOT surface errors to users. Cache writes
SHALL be atomic best-effort (temp file + rename), with write failures
ignored.

#### Scenario: an HTTP error returns none

- **WHEN** the crates.io API responds with a server error
- **THEN** the check SHALL return none without panicking

#### Scenario: an unreachable server returns none

- **WHEN** the API endpoint is unreachable (connection refused, timeout)
- **THEN** the check SHALL return none without panicking

#### Scenario: a corrupt cache falls back to a live fetch

- **WHEN** the cache file contains unparseable content
- **THEN** the check SHALL treat the cache as missing and fetch

#### Scenario: an unwritable cache directory still returns the result

- **WHEN** the cache directory cannot be created or written
- **THEN** the check SHALL still surface the fetched update info
- **AND** SHALL NOT panic

### Requirement: crates.io politeness

Requests SHALL carry a descriptive User-Agent identifying genesis. Rate-limit
responses (HTTP 403, 429) SHALL extend the cache TTL (doubling the default)
so retries are suppressed within the extended window.

#### Scenario: requests identify genesis

- **WHEN** the module issues a crates.io request
- **THEN** the request SHALL carry a User-Agent containing `genesis`

#### Scenario: a rate-limit response extends the cache TTL

- **WHEN** the crates.io API responds with 403 or 429
- **THEN** the check SHALL return none
- **AND** SHALL cache a miss with a doubled TTL so the next check within the
  extended window performs no HTTP request

### Requirement: Environment-aware skipping

The check SHALL skip entirely — before any IO — when `CI=true` (or `CI=1`)
is set or `GENESIS_NO_UPDATE_CHECK` is set to a non-empty value. An empty
`GENESIS_NO_UPDATE_CHECK` value SHALL NOT skip the check.

#### Scenario: CI skips the check before any IO

- **WHEN** `CI=true` is set and `check` is called
- **THEN** the check SHALL return none
- **AND** SHALL NOT write any cache entry

#### Scenario: the opt-out variable skips the check before any IO

- **WHEN** `GENESIS_NO_UPDATE_CHECK` is set to a non-empty value
- **THEN** the check SHALL return none without any IO

#### Scenario: an empty opt-out value does not skip

- **WHEN** `GENESIS_NO_UPDATE_CHECK` is set to an empty value
- **THEN** the check SHALL proceed to its normal fetch path

### Requirement: Scheme-agnostic version selection and notice

Version selection SHALL be scheme-agnostic: comparison SHALL be `current !=
latest`; yanked versions and semver pre-release versions SHALL be filtered
out of candidates. The notice SHALL be a single line naming the tool, both
versions, and the `cargo install <crate>` fix command.

#### Scenario: yanked and prerelease versions are never suggested

- **WHEN** the API response contains a newer pre-release version and a newer
  yanked version, but no newer stable version
- **THEN** the check SHALL return none

#### Scenario: calendar versions compare without semver assumptions

- **WHEN** the installed version is `2026.9.28` and the latest published
  stable version is `2026.10.12`
- **THEN** the check SHALL surface the update with latest `2026.10.12`

#### Scenario: the notice is a single actionable line

- **WHEN** `notice` is called with an update info for `wai`
- **THEN** the notice SHALL be exactly one line
- **AND** SHALL contain the tool name, latest version, installed version,
  and `cargo install wai`