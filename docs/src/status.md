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
## Distribution deviation (standard §1)

Standardization v0.4 §1 requires a five-target binary release matrix and a
README pointing at a prebuilt binary. **Genesis deviates:** it is a *library
crate*, not a CLI binary, so there are no binary archives to publish. Its
release slot is a tag-triggered `publish.yml` workflow (CI checks → crates.io).
The substitute install path — and the only one — is a crates.io dependency:

```toml
[dependencies]
genesis-vibes = "0.13"
```

This deviation is recorded here per §1 ("A repo that genuinely cannot meet
this (library crate, npm-only package) MUST record the deviation and its
substitute install path in docs/src/status.md").
