> *"Cuando todo era nada*
> *Era nada el principio*
> *Él era el Principio*
> *Y de la noche hizo luz*
> *Y fue el Cielo*
> *Y esto que está aquí"*
> — Vox Dei

# genesis-vibes

**TL;DR:** Shared Rust crate for cross-cutting CLI/AIX/self-healing infrastructure in the charly-vibes tool suite. Every tool depends on it instead of reimplementing the same conventions.

## What is genesis-vibes?

genesis-vibes is the shared foundation of the charly-vibes suite. It generalizes patterns that appear across multiple tools — structured CLI output, config management, diagnostics, scaffolding, test fixtures, and agent feedback — into a single crate with consistent conventions.

**Boundary rule:** if only one tool uses it, it does not belong in genesis. Domain logic (metrics, stores, engines, analysis) stays in each tool.

## Quick links

- [Getting Started](getting-started.md) — add genesis-vibes to your project
- [Modules Overview](reference/modules.md) — full module reference
- [Architecture](explanation/architecture.md) — how the modules fit together
- [Design Decisions](explanation/design-decisions.md) — rationale behind key trade-offs

## How-to guides

- [Using the Envelope](how-to/envelope.md) — structured CLI output
- [Building a CLI with Guide](how-to/guide.md) — verbosity, format, error handling
- [Adding a DoctorCheck](how-to/doctor.md) — diagnostic checks with auto-fix
- [Writing Tests with Fixture](how-to/fixture.md) — test scratch environments

## Modules at a glance

| Module | Status | Purpose |
| :--- | :--- | :--- |
| **envelope** | stable | Structured CLI output envelope |
| **guide** | stable | CLI scaffold: verbosity, output format, error handling |
| **suggestions** | stable | Self-healing error suggestions |
| **managed_block** | stable | Managed block injector |
| **aix** | stable | AIX artifact generation |
| **config** | stable | Shared config management |
| **fixture** | stable | Test scratch environments |
| **feedback** | stable | Agent issue reporting |
| **suite_linter** | stable | Suite-wide lint checks |
| **doctor** | new | Diagnostic framework with auto-fix |
| **cli** | new | CLI helpers (completions, version) |
| **status** | new | Cross-tool status dashboard |
| **scaffold** | new | Init scaffolding builder |
| **discovery** | new | Tool discovery via manifest |