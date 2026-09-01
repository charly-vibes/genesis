# Removed & Renamed APIs

**TL;DR:** This is the maintained source of the doc-drift blocklist in
`tests/doc_sync.rs`. When a public API is removed or renamed, add the *old*
call pattern as a row here — CI then fails if the old usage resurfaces in any
markdown doc.

## Why this exists

Between v0.4 and v0.6 the API moved (emit signatures, `ErrorSink`,
`ErrorResult`, output data handling), but the onboarding docs kept showing the
v0.4 snippets — nothing compiled them, so nothing failed. A documentation
review found 4 of 6 copy-paste snippets broken. This table, consumed by
`tests/doc_sync.rs::forbidden_api_patterns`, is the preventive layer: once a
pattern is listed here, any reappearance of the old usage in the book (or
README) fails `cargo test`.

## Maintenance contract

- **Add a row** whenever a public API is removed or renamed, quoting the old
  call shape as the pattern.
- The `pattern` cell must be a single backtick-quoted literal matched verbatim
  against every markdown file under `docs/` plus `README.md`.
- Keep the `reason` cell short and actionable: name the replacement.
- **Exception:** this page is excluded from the scan — it necessarily quotes
  the forbidden patterns.

## Forbidden patterns

| Pattern | Why it is forbidden |
| :--- | :--- |
| `.with_data(` | `Output::with_data` never existed; data goes in `Output::success(data)` |
| `.add_error(` | `ErrorSink::add_error` never existed; use `ErrorSink::handle` |
| `.emit_output(` | `ErrorSink::emit_output` never existed; use `ErrorSink::handle` |
| `ErrorSink::new()` | `ErrorSink::new` takes a tool_name argument |
| `emit(format, ` | `Output::emit` requires `cli_version` and `verbosity` args |
| `genesis-vibes = "0.4"` | stale version pin; Cargo.toml has moved on |
| `tag = "v0.4.0"` | stale git tag pin; use the current release tag |
| `process::exit() internally` | `maybe_print_version_json` returns bool; the caller exits |

## Related

- [CLI version ownership contract](../reference/modules.md#cli-version-ownership-contract)
- Drift guards: `tests/doc_sync.rs`; snippet mirrors: `tests/doc_examples.rs`
