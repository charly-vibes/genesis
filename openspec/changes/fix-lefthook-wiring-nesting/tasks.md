## 1. TDD — wiring placement

- [x] 1.1 RED: `ensure_wired_nests_block_inside_existing_commands_mapping` — block after `  commands:` line, content verbatim, no duplicate `commands:`
- [x] 1.2 RED: `ensure_wired_emits_commands_mapping_when_stage_lacks_one`
- [x] 1.3 RED: appended missing stage contains `  commands:` + block
- [x] 1.4 Flip `ensure_wired_inserts_block_directly_after_stage_key` to the new contract (or supersede by 1.1)
- [x] 1.5 GREEN: implement commands:-anchored insertion in `ensure_wired`
- [x] 1.6 Idempotence + unanchorable + missing-config tests stay green unchanged

## 2. Tidy + gates

- [x] 2.1 `cargo test` full suite green; clippy `-D warnings`; fmt
- [x] 2.2 `ah check` green; `openspec validate --strict`
- [x] 2.3 CHANGELOG entry
