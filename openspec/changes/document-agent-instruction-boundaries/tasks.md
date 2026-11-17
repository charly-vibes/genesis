# Tasks: document-agent-instruction-boundaries

Tracked as `genesis-ne0` in beads (base_commit `a6a5150`).

Meter: `just ci` (includes the mdBook docs build). `just book` does not exist.

## 1. Page

- [ ] 1.1 Write `docs/src/explanation/agent-instruction-boundaries.md` with
      the five sections named in the proposal (ownership, retirement,
      dialects, drift, facts-vs-procedures).
- [ ] 1.2 Include the fleet marker table (`DDL`, `WAI`, `BEADS`,
      `SPECODELIC`, `ah:managed`, `DONT`, `TURU`, retired `OPENSPEC`) with the
      file each one lives in.
- [ ] 1.3 Cite the OpenSpec 1.0.0 retirement (PR #578) and the
      fingerprint-gated removal as the worked example.
- [ ] 1.4 Add the rule a checker must follow: verify what the tool publishes
      (block **or** artifact path), scoped to repos that use the tool.

## 2. Wiring

- [ ] 2.1 Add the page to `docs/src/SUMMARY.md` under Explanation.
- [ ] 2.2 Trim `src/managed_block.rs` module docs to the API contract plus a
      pointer to the page; keep the provenance footer detail.
- [ ] 2.3 Add the pointer to `docs/src/reference/modules.md` under
      `managed_block`.

## 3. Verify

- [ ] 3.1 `just book` (mdBook build) succeeds with no broken links.
- [ ] 3.2 `cargo test` unchanged (`managed_block` tests untouched).
- [ ] 3.3 Cross-link from ddl's `fix-agent-instrumentation-check` design on
      archive.

## 4. Archive

- [ ] 4.1 `openspec archive document-agent-instruction-boundaries` once the
      managed-block spec requirement is merged.