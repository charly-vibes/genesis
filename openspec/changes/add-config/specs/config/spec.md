# config delta

## ADDED Requirements

### Requirement: ConfigError type

genesis SHALL provide a `ConfigError` enum covering the failure modes of
config file operations.

#### Scenario: missing file returns ConfigError::MissingFile

- **WHEN** a tool calls `ConfigFile::read()` on a nonexistent path
- **THEN** genesis SHALL return `ConfigError::MissingFile`
- **AND** the error SHALL include the expected path

#### Scenario: malformed TOML returns ConfigError::ParseError

- **WHEN** a config file exists but contains invalid TOML
- **THEN** genesis SHALL return `ConfigError::ParseError`
- **AND** the error SHALL include the parser's diagnostic

#### Scenario: validation failure returns ConfigError::ValidationError

- **WHEN** a config file parses but fails `validate()`
- **THEN** genesis SHALL return `ConfigError::ValidationError`
- **AND** the error SHALL include the validation message

#### Scenario: missing config suggests init command

- **WHEN** `ConfigError::MissingFile` is constructed
- **THEN** `to_suggestion()` SHALL return `Suggestion::Fix`
- **AND** the fix command SHALL be `<tool> init`

#### Scenario: validation error suggests doctor --fix

- **WHEN** `ConfigError::ValidationError` is constructed
- **THEN** `to_suggestion()` SHALL return `Suggestion::Fix`
- **AND** the fix command SHALL be `<tool> doctor --fix`

### Requirement: ConfigFile trait

genesis SHALL provide a `ConfigFile` trait that tools implement to declare
their config path, parse, and serialize logic.

#### Scenario: tool defines config with default impl

- **WHEN** a tool's config struct derives `Serialize` + `Deserialize` + `Default`
- **AND** the tool implements `ConfigFile` with `fn path() -> PathBuf`
- **THEN** the default `read()` impl SHALL read the file, parse with serde, and return the struct
- **AND** the default `write()` impl SHALL serialize with serde and write to the path

#### Scenario: validate returns no errors by default

- **WHEN** a tool's config struct does not override `validate()`
- **THEN** `validate()` SHALL return an empty vec

### Requirement: ConfigRegistry

genesis SHALL provide a `ConfigRegistry` where tools register their config
type at startup.

#### Scenario: tool registers at startup

- **WHEN** a tool calls `registry::register::<MyConfig>("my-tool")`
- **THEN** the registry SHALL store the tool name and config type metadata
- **AND** `registry::registered_tools()` SHALL include `"my-tool"`

#### Scenario: get returns the parsed config

- **WHEN** a tool calls `registry::get("my-tool")` and the config file exists
- **THEN** the registry SHALL read, parse, and return the config
- **AND** SHALL return `ConfigError::MissingFile` if the file doesn't exist

### Requirement: ConfigStore

genesis SHALL provide a `ConfigStore` wrapping the registry for discovery
and batch operations.

#### Scenario: discover finds all registered configs in a repo

- **WHEN** `ConfigStore::discover(repo_root)` is called
- **THEN** it SHALL walk the repo root looking for registered tool markers
- **AND** SHALL return a map of found tool names to their parsed configs
- **AND** SHALL skip tools whose marker doesn't exist (not an error)

#### Scenario: validate_all runs all registered validators

- **WHEN** `ConfigStore::validate_all()` is called
- **THEN** it SHALL call `validate()` on each found config
- **AND** SHALL collect and return all validation messages

#### Scenario: managed_block generates a table of found and missing configs

- **WHEN** `ConfigStore::managed_block()` is called
- **THEN** it SHALL return a markdown table with columns: Tool, Path, Status
- **AND** each registered tool SHALL have a row
- **AND** each row SHALL show ✅ found or ❌ missing
- **AND** the block SHALL end with a next-step suggestion for any missing configs

#### Scenario: managed_block markers follow the <!-- CONFIG:START/END --> convention

- **WHEN** `ConfigStore::managed_block()` is called
- **THEN** the output SHALL start with `<!-- CONFIG:START -->`
- **AND** SHALL end with `<!-- CONFIG:END -->`