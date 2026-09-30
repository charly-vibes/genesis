# Status

**stable** — the shared foundation crate for the charly-vibes suite; every
in-org CLI depends on it. Release cadence follows consumer needs.

## What it provides

- `guide` — progressive CLI verbosity/formatting (`-v/-vv/-vvv`, `--json` envelopes)
- `cli` — completions, version JSON, shared CLI plumbing
- `config` — `ConfigFile` trait and shared config loading
- `fixture` — deterministic test fixtures
- `aix` — AGENTS.md block generation
- `envelope`, `doctor`, `status`, `suggestions`, `suite_linter` — CLI diagnostics and self-healing
- `git_hooks` — multi-scope `core.hooksPath` resolution (v0.8.3+)

## Consumers

dont, dulce-de-leche (ddl), testaruda, espectacular (ah), vampiro, whisper (turu),
wai, and others — see each repo's README badge row.

## Maturity notes

- Core modules (envelope, doctor, cli, config) are stable and widely exercised.
- `git_hooks` multi-scope resolution landed in v0.8.3 and is verified against
  global/local shim scenarios.
- API surface may grow; breaking changes go through a minor-version bump with
  consumer CI as the safety net (fleet conformance tests).