# Tasks: document-fleet-adoption-review

Tracked as `genesis-7mi` in beads (base_commit `3bffa47`).

Meter: `just ci` (includes the mdBook docs build).

## 1. Page

- [ ] 1.1 Write `docs/src/explanation/fleet-adoption.md` with the sections:
      the adoption rule; the adoption matrix; the findings with ids; the
      owner-lanes; the rejected candidates; the verification record.
- [ ] 1.2 The matrix: tool → genesis version → modules used → verification
      state (`verified` / `dep-only` / `unverified`); include the 18 CLIs,
      exclude `fotos` with the reason stated.
- [ ] 1.3 The findings: `F-01`… with location, one-line claim, lane
      (genesis / tool repo / fleet contract), and the reproduction command.
- [ ] 1.4 The corrected counts with their derivations: atomic-write sites
      (12 in 7 tools), `genesis::doctor` adopters (9), `genesis::feedback`
      adopters (8), `genesis::status` adopters (5), tools with no genesis
      dependency (4).
- [ ] 1.5 The verification record: which findings an independent reader
      confirmed, which came back below its confidence gate, which are
      unverified, and the measured false-positive rate.
- [ ] 1.6 The rejected candidates section: wild's v1 envelope, podras' health
      signals, funkalization's tracer model — each with the reason it stays
      in the tool.

## 2. Wiring

- [ ] 2.1 Link the page from `docs/src/SUMMARY.md` under Explanation.
- [ ] 2.2 Cross-link the page from `docs/src/reference/modules.md` (the
      adoption matrix names the modules).
- [ ] 2.3 Reference the OpenSpec changes that carry the findings, by change
      name.

## 3. Follow-up (filed at approval, not implemented here)

- [ ] 3.1 One bd issue per finding, quoting its `F-` id and naming its lane.
- [ ] 3.2 Close `genesis-lu6` (register the `.espectacular` detector) once
      `populate-tool-registry` lands.
- [ ] 3.3 `versions.ddl.toml` — no entry (docs only).
