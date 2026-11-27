---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references: []
---

# Existing assertions — genesis

Scan of the genesis codebase for Antithesis SDK instrumentation (imports of
the Antithesis SDK and calls to `assert_always!`, `assert_sometimes!`,
`assert_reachable!`, `assert_unreachable!` or their non-macro equivalents),
per the antithesis-research workflow.

**No Antithesis SDK assertions were found.** No file under `src/` imports
any Antithesis/`antithesis` crate; `Cargo.toml` has no related dependency
(deps: serde, serde_json, thiserror, clap, clap_complete, strsim, toml,
regex, tempfile, optional semver/ureq; dev-deps clap/serde_json). There is
no existing instrumentation for the four seams analyzed in this ticket;
every property in the catalog would be new instrumentation.

Closest existing equivalents are ordinary `#[test]` assertions (inventory in
[sut-analysis.md](sut-analysis.md) § Existing test coverage), which pin the
implemented semantics the catalog properties refine — including
`test_inject_end_marker_before_start_is_not_corrupted`
(`src/managed_block/tests/splice.rs:71`), which pins stray-END survival as
intended behavior and therefore bounds how the marker-uniqueness property
may be stated.
