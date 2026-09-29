# Tasks

## 1. Multi-scope resolution (TDD)

- [x] 1.1 RED: failing tests — global/system scope honored, local > global precedence, `effective_hooks_dir()` scope reporting, empty-string `Disabled`
- [x] 1.2 GREEN: `HooksDirScope` + `EffectiveHooksDir` types, scope-walking resolution, `resolve_hooks_dir()` delegation
- [x] 1.3 Docs: modules.md git_hooks section, CHANGELOG [Unreleased]

## 2. Validation

- [x] 2.1 `openspec validate --strict`, full test suite, clippy/fmt, `ah check`
