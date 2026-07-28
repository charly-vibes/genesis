# guide delta

## ADDED Requirements

### Requirement: Guide builder

genesis SHALL provide a `Guide` builder that assembles the five design principles into a single entry point.

#### Scenario: guide builds with tool name and version

- **WHEN** `Guide::new("my-tool", "0.1.0").build()` is called
- **THEN** the guide SHALL have tool_name = "my-tool" and version = "0.1.0"
- **AND** the `SuggestionEngine` SHALL be initialized with default threshold

#### Scenario: guide registers commands for typo detection

- **WHEN** `Guide::new("my-tool", "0.1.0").commands(&["init", "check"]).build()` is called
- **THEN** the `CommandRegistry` SHALL contain "init" and "check"
- **AND** a typo like "chek" SHALL produce a `DidYouMean` suggestion

#### Scenario: guide builds with optional config

- **WHEN** `Guide::new("my-tool", "0.1.0").config::<MyConfig>().build()` is called
- **THEN** the guide SHALL initialize a `ConfigStore` for `MyConfig`
- **AND** the tool's error sink SHALL use `ConfigError::to_suggestion()` for config errors

### Requirement: Output type

genesis SHALL provide an `Output<T>` type for guided command results.

#### Scenario: output carries data and next step

- **WHEN** a command handler returns `Output { data: "result", next_step: Some(suggestion), warnings: vec![], verbosity: 1 }`
- **THEN** the guide SHALL print data to stdout
- **AND** SHALL print the next_step footer to stderr

#### Scenario: output with --json flag produces envelope

- **WHEN** a command returns `Output { data: some_data, .. }`
- **AND** the tool is run with `--json`
- **THEN** the guide SHALL serialize the output through `genesis::envelope::Envelope`
- **AND** the envelope SHALL have `ok: true` and `envelope_kind` matching the command

#### Scenario: output with verbosity filter hides warnings

- **WHEN** a command returns `Output { verbosity: 2, warnings: vec![...] }`
- **AND** the tool is run with `-q` (verbosity 0)
- **THEN** the warnings SHALL NOT be printed

### Requirement: ErrorSink

genesis SHALL provide an `ErrorSink` for self-healing error handling.

#### Scenario: error prints with fix footer

- **WHEN** `ErrorSink::new(scratch: true, suggest: true, context: false).handle(err)` is called
- **THEN** the error SHALL be printed to stderr
- **AND** a `Suggestion::Fix` footer SHALL be printed
- **AND** the error SHALL be written to the error scratch file

#### Scenario: error without fix suggests feedback only when configured

- **WHEN** an error has no `Suggestion::Fix` available
- **AND** `ErrorSink.feedback_subcommand` is `Some("my-tool feedback")`
- **THEN** the error sink SHALL print `Feedback: my-tool feedback bug --from-last-error`

#### Scenario: error without fix does not suggest feedback when not configured

- **WHEN** an error has no `Suggestion::Fix` available
- **AND** `ErrorSink.feedback_subcommand` is `None`
- **THEN** the error sink SHALL NOT print any feedback suggestion

### Requirement: Verbosity

genesis SHALL provide a `Verbosity` enum with three tiers.

#### Scenario: quiet mode suppresses suggestions

- **WHEN** `Verbosity::Quiet` is active
- **THEN** only errors SHALL be printed
- **AND** next-step suggestions SHALL be suppressed

#### Scenario: verbose mode shows warnings

- **WHEN** `Verbosity::Verbose` is active
- **THEN** warnings and context SHALL be printed alongside results