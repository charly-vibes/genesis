## 1. Crate skeleton
- [x] 1.1 Populate the existing `genesis` repo with `Cargo.toml`, `src/lib.rs`, `justfile`, `lefthook.yml`, `_typos.toml`, `book.toml`, `llms.txt`/`llm.txt`, `AGENTS.md`.
- [x] 1.2 Define `src/lib.rs` re-exporting the six modules (plus `config`).
- [x] 1.3 Add CI (fmt-check + clippy + test + build-release) mirroring the suite `just ci`.
- [x] 1.4 Tag `v0.1.0` once stubs compile.

## 2. Extract `envelope` (supersedes dont-2j6o)
- [x] 2.1 Port `dont/src/envelope.rs` into `genesis/src/envelope.rs` carrying dont-2j6o's design notes verbatim.
- [x] 2.2 Make the `data` field generic over `T: Serialize`.
- [x] 2.3 Add conformance tests for the envelope shape.
- [x] 2.4 Close `dont-2j6o` as superseded — children re-pointed at per-repo `adopt-genesis` proposals.

## 3. Extract `suggestions` (donor: wai)
- [x] 3.1 Port `wai/src/suggestions.rs` into `genesis/src/suggestions.rs` (Suggestion enum, SuggestionEngine, suggest_typo/suggest_order — removed suggest_context as not general enough).
- [x] 3.2 Add a `footer()` helper that formats a `Suggestion` as the "→ Run: …" footer string.
- [x] 3.3 Add `CommandRegistry` so tools register valid commands and get typo detection for free.

## 4. Extract `managed_block` (donors: wai/dont/espectacular)
- [x] 4.1 Port the `<!-- …:START -->`/`<!-- …:END -->` injector from `wai/src/managed_block.rs` into `genesis/src/managed_block.rs`.
- [x] 4.2 Generalize the block registry via `BlockRegistry` so any tool can register a named block (WAI, OPENSPEC, DONT, ah:managed, …).
- [x] 4.3 Preserve the slim-block Layer-1 progressive-disclosure behavior.

## 5. Build `feedback` (new)
- [x] 5.1 Implement the redactor (whole-body, value-not-key-substring matching, `git_remote` host/path reduction).
- [x] 5.2 Implement the context-bundle serializer.
- [x] 5.3 Implement the error-scratch JSONL writer (best-effort, read-only/temp fallback, never shadows the real error).
- [x] 5.4 Implement the `gh` invocation + fallback ladder (missing/unauthed/labels/no-network/permission).

## 6. Build `suite_linter` (wai-bdqw.8)

The suite_linter is an **orchestrator, not a monolith**. Each tool defines
its own checks via the `LintCheck` trait; genesis just runs them.

- [x] 6.1 Define the `LintCheck` trait (fn signature, result type, fix fn).
- [x] 6.2 Define a `LinterRegistry` where tools register their checks at startup.
- [x] 6.3 Implement `LinterRegistry::run_all()` — iterates, runs, collects results.
- [x] 6.4 Implement `LinterRegistry::run_named(name)` — run a single check by name.
- [x] 6.5 Add `LintResult` type with severity (advisory/warning/error) and optional fix command.

Tool-specific checks (testaruda.toml schema, pretender.toml presence, ah/dont
wiring, badge↔block match) belong in each tool's own `doctor` command, not
in genesis. Each tool registers its checks with the `LinterRegistry`.

## 7. AIX generation (`aix` module)
- [ ] 7.1 Extract `llms.txt`/`llm.txt` generation from wai into genesis. *(Not yet — full LLM generation deferred; `aix.rs` has only `agents_block` helper.)*
- [x] 7.1b Genesis bootstrap: hand-written `AGENTS.md`, `llms.txt`, `llm.txt` exist.
- [x] 7.2 Provide an `agents_block(name, body)` helper for managed-block injection.

## 8. Stabilize and pin
- [x] 8.1 Tag `v0.1.0` with the six modules stable.
- [ ] 8.2 Verify `.wai/projects/genesis-foundation/research/tool-craft.md` Appendix A accuracy once dependents adopt (genesis-9o5 filed, P3).
- [ ] 8.3 Open the per-repo adoption proposals (one per tool repo) — unblocked now that v0.1.0 is tagged.
