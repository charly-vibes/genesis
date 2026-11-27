# FAQ & troubleshooting

Frequently asked questions and recurring failure modes for consumers of
genesis-vibes.

## Do I need genesis if I already use one of the suite CLIs?

As a user, no — the CLI you use (`dont`, `wai`, `testaruda`, `ah`, …) already
depends on genesis transitively, which is why their output envelopes, `-v`/`--json`
flags, and doctor checks all behave identically. You need a direct dependency
only if you are *building* a new tool for the suite.

## Is there a binary to install?

No. genesis is a lib crate published to crates.io. Install it by adding it to
your consumer crate's `[dependencies]` (see the README Install section). The
library-crate deviation from standardization §1 is recorded in
[status.md](status.md).

## What files does genesis write or modify?

Nothing on its own — it is a library driven by your CLI's code. Callers opt in
per operation: managed blocks in AGENTS.md-style markdown, the
`.genesis/tools.toml` discovery manifest, installed git hooks, scaffolded
config files. If your tool writes a file you did not expect, check which
genesis module your `init`/install path calls.

## Can I use genesis for my tool's domain logic?

No. genesis owns cross-cutting infrastructure only (envelopes, verbosity,
doctor checks, config plumbing, hooks). Boundary rule: if only one tool uses
it, it does not belong in genesis. Keep domain logic in your own crate; see
the README's "When to use" section.

## Why do my envelopes claim `ok: true` for failures?

A failed output must be routed through `Envelope::success` (or the error
constructor) with `EnvelopeKind::Error`, which makes `ok` false. If `ok` is
true while the kind is `Error`, you built the kind wrong — the
`ok_for(kind)` invariant (genesis-r13) maps the `Error` kind to `ok: false`.

## Why does my JSON output look human-readable (or vice versa)?

`CliFormat::format()` auto-detects: stdout is a TTY → Human; piped/redirected
→ Json. Force one explicitly with `--json` or `--human` (or set
`OutputFormat` directly in code).

## Doctor report shows failures on a clean checkout — why?

`DoctorRunner::run(&repo, fix)` runs your `DoctorCheck` implementations
against the given repo root. Checks that assume prior state (installed hooks,
registered tools, config files) fail on a fresh clone until your tool's
`init` has run. Verify the repo root you pass and whether `fix: true` (auto-fix)
is intended for that invocation.
