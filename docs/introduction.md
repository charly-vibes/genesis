> *"Cuando todo era nada*
> *Era nada el principio*
> *Él era el Principio*
> *Y de la noche hizo luz*
> *Y fue el Cielo*
> *Y esto que está aquí"*
> — Vox Dei

# genesis

Shared crate for cross-cutting CLI/AIX/self-healing infrastructure.

## Modules

- **envelope** — structured CLI output envelope (port from dont)
- **suggestions** — self-healing error suggestions (port from wai)
- **managed_block** — managed block injector (port from wai/dont/espectacular)
- **aix** — AIX artifact generation helpers
- **config** — shared config management (ConfigFile, ConfigRegistry, ConfigStore)
- **guide** — CLI scaffold (Verbosity, CliVerbosity, OutputFormat, CliFormat, Output, ErrorSink, GuideBuilder, Guide)
- **fixture** — test scratch environments and runners
- **feedback** — agent issue reporting (handle_feedback, FeedbackArgs)
- **suite_linter** — suite-wide lint checks
- **doctor** — diagnostic framework with auto-fix (DoctorCheck, DoctorRunner, DoctorReport)
- **cli** — CLI helpers (generate_completions, maybe_print_version_json)
- **status** — cross-tool status dashboard (StatusContributor, StatusBuilder)
- **scaffold** — init scaffolding builder (Scaffold)
- **discovery** — tool discovery via .genesis/tools.toml manifest (scan, register, unregister, Manifest)