# Project status brief

## Purpose / current state

none

## Focus

none

## In flight

- genesis-ntg — Epic: Agent Value Alignment — cross-cutting findings across all 7 tools (status: open; priority: 0; age: 109d)
- genesis-azy — Research: Hegel property-based testing plan (antithesis-research) (status: open; priority: 1; age: 2d)
- genesis-b9p — docs: apply README front-door standard v0.4 (§5.1) (status: open; priority: 1; age: 2d)
- genesis-otx — Implement Hegel property tests (status: open; priority: 1; age: 2d; blocked by: genesis-azy)
- genesis-3ps — CI eval-gate: wire receipt check + scenario replay into downstream CI (status: open; priority: 2; age: 48d)
- genesis-9h9 — pre-push hook broken: lefthook.yml passes --advisory-until but installed pretender CLI rejects it (status: open; priority: 2; age: 33d)
- genesis-c9x — feedback: ErrorRecord has no error-message field — auto-reported GH issues have an empty '## Error' body (status: open; priority: 2; age: 35d)
- genesis-dvf — testaruda: wire feedback subcommand via genesis::feedback (status: open; priority: 2; age: 62d)
- genesis-fkt — add-artifact-provenance: per-repo adoption — managed-block footers + drift lint (wai, dont, pretender, testaruda) (status: open; priority: 2; age: 48d)
- genesis-lu6 — Register .espectacular detector in genesis tools (status: open; priority: 2; age: 41d)
- genesis-tpf — Add shared git interface module to genesis (status: open; priority: 2; age: 40d)
- genesis-tpf.8 — Migrate wai close.rs uncommitted-files onto genesis::git, fix rename defect (work in charly/wai) (status: in_progress; priority: 2; age: 40d)
- genesis-2ex — update-check: feature-gated genesis::update_check module + wiring in dependent CLIs (status: in_progress; priority: 3; age: 49d)
- genesis-a1h — openspec 1.x upgrade blocked: 10 validation failures under @fission-ai/openspec@1.14.2 (status: open; priority: 3; age: 33d)
- genesis-le7 — refactor(cli): return io::Result<bool> from maybe_print_version_json_from (status: deferred; priority: 3; age: 78d)
- genesis-orq — Migrate wai way/hooks.rs onto genesis::git_hooks (work in charly/wai) (status: open; priority: 3; age: 51d)
- genesis-q4k — Migrate pretender onto genesis::git_hooks (work in charly/pretender) (status: open; priority: 3; age: 51d)
- genesis-tpf.6 — Migrate whisper private-zone checks onto genesis::git (work in charly/whisper) (status: in_progress; priority: 3; age: 40d)
- genesis-tpf.7 — Migrate dulce dot_ddl tracked check onto genesis::git (work in charly/dulce-de-leche) (status: in_progress; priority: 3; age: 40d)

## Blocked / waiting

- genesis-otx — Implement Hegel property tests (status: open; priority: 1; age: 2d; blocked by: genesis-azy)

## Next / ready

- genesis-ntg — Epic: Agent Value Alignment — cross-cutting findings across all 7 tools (status: open; priority: 0; age: 109d)
- genesis-azy — Research: Hegel property-based testing plan (antithesis-research) (status: open; priority: 1; age: 2d)
- genesis-b9p — docs: apply README front-door standard v0.4 (§5.1) (status: open; priority: 1; age: 2d)
- genesis-3ps — CI eval-gate: wire receipt check + scenario replay into downstream CI (status: open; priority: 2; age: 48d)
- genesis-9h9 — pre-push hook broken: lefthook.yml passes --advisory-until but installed pretender CLI rejects it (status: open; priority: 2; age: 33d)
(8 more omitted)

## Parked

none

### Needs re-triage

none

## Recent

- 4d2572031c8426261bf2f797ac895221f1e2953e beads: close genesis-ne0 and genesis-mpx (2026-11-17T22:42:15Z)
- e9eff8e8ca6a492a5477427336a32d7dd34f51e8 beads: close genesis-ne0 and record the gate repair (2026-11-17T22:41:16Z)
- 32970f61d165879e47256bd6a816734f3b06ad2c chore(gates): repair pre-existing fmt/clippy drift and refresh the gate block (genesis-mpx) (2026-11-17T22:41:01Z)
- 2cc020f007e4bc53b176e182ba33e0bbfc568a41 docs: publish the agent-instruction boundary contract (genesis-ne0) (2026-11-17T22:40:35Z)
- a6a5150a622e165c1ccd14c2e72975ad2499caf0 docs: apply issue-review fixes to README/release tickets (2026-11-16T11:57:19Z)
(195 more omitted)

## Health + confidence

Health: **amber** — basis: signals — owner: none — reason: orphan_commits: 34 commits in the 14-day window link no tracker id or open work-item path: 01e0d946da4051b77a9fa7890dc87b4632286388 (fix(quality): TestUnwrap import for scratch test helper); 02cda6b2b8da3330d6664d4d8c26388a1a3cc25e (docs(brief): pretender debt fix brief — orchestrator artifact (docs-only; gate blocked by pre-existing debt)); 06d7edff1fbe49308d99e6154c2882cf9971bac9 (fix(quality): real assert! macros in fixture tests (assert_* helpers count as 0)); 16a1680a8d7096daee97304ced2d8bf3b5a58d80 (refactor(quality): extract helpers in discovery, managed_block, feedback/context, redactor); 1f55670adb8ae0e8f8351da85c3855625b494caa (refactor(quality): split update_check cache layer; extract fetch/persist helpers); 25b26fb57c581efc22a3609f640aebb4b03c402d (refactor(quality): split evals report-contract tests (tests_report.rs); prune dead imports); 3c2094114c3fadf9fdcf9731762a0d7dc54be5a5 (refactor(quality): extract round-trip assertion from lifecycle test); 4f95319dbac842dc6d4fbb152a561b6ebffbac7b (fix(quality): move append_test_records inside tests mod (cfg(test) scoping)); 5022074f349ea4c90817fc41df591aa7bd062be1 (refactor(quality): split config into config/{error,validation,registry,store}; guide+config helpers; split config tests); 50753cfd4de48f4dfb2a6f659972fa730ccd6d2d (style: cargo fmt after helper insertions); 55d50bdabb6b54c6f3d20b3475a8f32d81656676 (refactor(quality): extract inline test mods to role-annotated files; de-mut builders); 56cb9d21366e8ad2fd9df068a193570b4d1b376a (refactor(quality): split status tests (tests_builder.rs) — clears 8-member lazy cluster); 57fd5e470eeace2ee1f4d42e237f9d84b6927d71 (fix(quality): clear test-role pretender debt in tests/ (gate mode)); 6830d36b76dd0bad1807614a250fef080a6a169b (fix(quality): convert .expect sites in extracted test files (broken regex in prior batch)); 6884e00eb12ccf7b8472a4a9f6eb4d4010d4d1b6 (refactor(quality): final scoped pretender-gate sweep — doctor/git_hooks/feedback splits + helper decomposition); 756c4e3b324f873d9a167bef72f5f6a1271bca29 (docs: file README front-door and release tickets); 766c526ac9669c5f82a824a33214e4c87ec047f9 (refactor(quality): split ManagedBlockDrift into suite_linter/drift.rs; de-cluster tests); 7d7a57b63fbe70a5376d63693bdfc13b8d15ee3a (refactor(quality): decompose config lifecycle test; hoist OtherConfig fixture); 7f7e2ebc0727a2603456e4152a8c1cc152224771 (refactor(quality): split Doctor bridge from status; extract suggestion/status helpers); 8ca1ef35f34d6a26f4122ff38a7e5cb8f55fc02d (refactor(quality): scratch timestamp helpers split; test records helper; require-conversion); 8de4a248dca3dcc1876c1ea9fbc6958e38f35dc0 (refactor(quality): split evals receipt tests (tests_receipt.rs); no-nulls walker helper); a6a5150a622e165c1ccd14c2e72975ad2499caf0 (docs: apply issue-review fixes to README/release tickets); a6ededa7b411d5cde566dbfdb846122500b3ccb4 (refactor(quality): split git and fixture into module dirs; extract helpers); ac69ea1ac94caea8d87fba5ac8afb57116dd3056 (docs: apply issue-review fixes to README/release tickets); afa0ae2db3b985e93d554afe8fd8d6c68652b9c5 (refactor(quality): split suite_linter drift module; de-cluster tests; scaffold build decomposition); b7c228a7142c64776854bf018c733ee3af644f81 (refactor(quality): gh error-mapping table + spawn/parse helpers; scaffold cleanup); baf29fae5b9439cb72f64d86ed78b3e3ec26526a (refactor(quality): scaffold build decomposition + mut-free path partitioning); c3a1acbdcd40784942a313f8ea1247763995d77e (refactor(quality): extract inline test mods from 9 large src modules); df3a22842b90c1c089675a840d77d194f1b8c9df (refactor(quality): split git.rs into git/{error,runner,changes,query}; extract helpers); e0795a4f8e568aed6d513c72f76b4064ce33fb74 (refactor(quality): split guide into guide/{verbosity,format,output,error_sink,run}); ed75734d50d5f6dd70ff9b3dec57a8b12dd8ac81 (refactor(quality): split evals into evals/{taxonomy,outcome,scenario,checks}); f0e414476ba9b6bbba2091ddecf806e9ce7fa704 (refactor(quality): split aix into module dir; extract helpers in gen-aix and cli); f489190e84a26255a07080f8693964862a95c4aa (fix(quality): build_registry takes owned Vec<String>); fc69628b5927b3706473a32725c55dec31f951f8 (refactor(quality): split guide tests (guide/tests_guide.rs) for file-lines budget); spec_change_without_bead: openspec changes link no tracker id: `document-agent-instruction-boundaries` changed 1 days ago
- signal: orphan commits — 34 commits in the 14-day window link no tracker id or open work-item path: 01e0d946da4051b77a9fa7890dc87b4632286388 (fix(quality): TestUnwrap import for scratch test helper); 02cda6b2b8da3330d6664d4d8c26388a1a3cc25e (docs(brief): pretender debt fix brief — orchestrator artifact (docs-only; gate blocked by pre-existing debt)); 06d7edff1fbe49308d99e6154c2882cf9971bac9 (fix(quality): real assert! macros in fixture tests (assert_* helpers count as 0)); 16a1680a8d7096daee97304ced2d8bf3b5a58d80 (refactor(quality): extract helpers in discovery, managed_block, feedback/context, redactor); 1f55670adb8ae0e8f8351da85c3855625b494caa (refactor(quality): split update_check cache layer; extract fetch/persist helpers); 25b26fb57c581efc22a3609f640aebb4b03c402d (refactor(quality): split evals report-contract tests (tests_report.rs); prune dead imports); 3c2094114c3fadf9fdcf9731762a0d7dc54be5a5 (refactor(quality): extract round-trip assertion from lifecycle test); 4f95319dbac842dc6d4fbb152a561b6ebffbac7b (fix(quality): move append_test_records inside tests mod (cfg(test) scoping)); 5022074f349ea4c90817fc41df591aa7bd062be1 (refactor(quality): split config into config/{error,validation,registry,store}; guide+config helpers; split config tests); 50753cfd4de48f4dfb2a6f659972fa730ccd6d2d (style: cargo fmt after helper insertions); 55d50bdabb6b54c6f3d20b3475a8f32d81656676 (refactor(quality): extract inline test mods to role-annotated files; de-mut builders); 56cb9d21366e8ad2fd9df068a193570b4d1b376a (refactor(quality): split status tests (tests_builder.rs) — clears 8-member lazy cluster); 57fd5e470eeace2ee1f4d42e237f9d84b6927d71 (fix(quality): clear test-role pretender debt in tests/ (gate mode)); 6830d36b76dd0bad1807614a250fef080a6a169b (fix(quality): convert .expect sites in extracted test files (broken regex in prior batch)); 6884e00eb12ccf7b8472a4a9f6eb4d4010d4d1b6 (refactor(quality): final scoped pretender-gate sweep — doctor/git_hooks/feedback splits + helper decomposition); 756c4e3b324f873d9a167bef72f5f6a1271bca29 (docs: file README front-door and release tickets); 766c526ac9669c5f82a824a33214e4c87ec047f9 (refactor(quality): split ManagedBlockDrift into suite_linter/drift.rs; de-cluster tests); 7d7a57b63fbe70a5376d63693bdfc13b8d15ee3a (refactor(quality): decompose config lifecycle test; hoist OtherConfig fixture); 7f7e2ebc0727a2603456e4152a8c1cc152224771 (refactor(quality): split Doctor bridge from status; extract suggestion/status helpers); 8ca1ef35f34d6a26f4122ff38a7e5cb8f55fc02d (refactor(quality): scratch timestamp helpers split; test records helper; require-conversion); 8de4a248dca3dcc1876c1ea9fbc6958e38f35dc0 (refactor(quality): split evals receipt tests (tests_receipt.rs); no-nulls walker helper); a6a5150a622e165c1ccd14c2e72975ad2499caf0 (docs: apply issue-review fixes to README/release tickets); a6ededa7b411d5cde566dbfdb846122500b3ccb4 (refactor(quality): split git and fixture into module dirs; extract helpers); ac69ea1ac94caea8d87fba5ac8afb57116dd3056 (docs: apply issue-review fixes to README/release tickets); afa0ae2db3b985e93d554afe8fd8d6c68652b9c5 (refactor(quality): split suite_linter drift module; de-cluster tests; scaffold build decomposition); b7c228a7142c64776854bf018c733ee3af644f81 (refactor(quality): gh error-mapping table + spawn/parse helpers; scaffold cleanup); baf29fae5b9439cb72f64d86ed78b3e3ec26526a (refactor(quality): scaffold build decomposition + mut-free path partitioning); c3a1acbdcd40784942a313f8ea1247763995d77e (refactor(quality): extract inline test mods from 9 large src modules); df3a22842b90c1c089675a840d77d194f1b8c9df (refactor(quality): split git.rs into git/{error,runner,changes,query}; extract helpers); e0795a4f8e568aed6d513c72f76b4064ce33fb74 (refactor(quality): split guide into guide/{verbosity,format,output,error_sink,run}); ed75734d50d5f6dd70ff9b3dec57a8b12dd8ac81 (refactor(quality): split evals into evals/{taxonomy,outcome,scenario,checks}); f0e414476ba9b6bbba2091ddecf806e9ce7fa704 (refactor(quality): split aix into module dir; extract helpers in gen-aix and cli); f489190e84a26255a07080f8693964862a95c4aa (fix(quality): build_registry takes owned Vec<String>); fc69628b5927b3706473a32725c55dec31f951f8 (refactor(quality): split guide tests (guide/tests_guide.rs) for file-lines budget)
- signal: spec change without bead — openspec changes link no tracker id: `document-agent-instruction-boundaries` changed 1 days ago

## Sources & freshness

| source | status | observed_at | staleness | detail |
| --- | --- | --- | --- | --- |
| beads | present | 2026-11-17T22:42:15Z | fresh | 32 malformed line(s) skipped in .beads/issues.jsonl |
| git | present | 2026-11-17T22:42:15Z | fresh | none |
| openspec | present | 2026-11-17T19:50:38Z | fresh | none |
| whisper | present | 2026-10-09T23:44:17Z | stale | none |
| curation | present | 2026-11-18T21:12:09Z | fresh | none |

Curation: updated_at 2026-11-18T21:12:09Z; confirmed_at none.

Freshness: generated_at 2026-11-18T21:12:09Z; per-source observed_at and staleness are listed above.
