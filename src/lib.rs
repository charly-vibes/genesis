//! genesis-vibes — shared crate for cross-cutting CLI/AIX/self-healing infrastructure.
//!
//! Modules, each extracted only when >=2 tools need it:
//!
//! - `aix`           — AIX artifact generation (port from wai)
//! - `cli`           — CLI helpers (completions, version-json pre-parse)
//! - `config`        — shared config management
//! - `discovery`     — tool discovery via `.genesis/tools.toml` manifest (new)
//! - `doctor`        — diagnostic framework with auto-fix support (new)
//! - `envelope`      — structured CLI output envelope (port from dont)
//! - `feedback`      — agent issue reporting (new)
//! - `fixture`       — test scratch environments and runners (new)
//! - `guide`         — CLI scaffold for building guiding tools (new)
//! - `managed_block` — managed block injector (port from wai/dont/espectacular)
//! - `scaffold`      — init scaffolding for standardized setup (new)
//! - `status`        — cross-tool status/prime dashboard (new)
//! - `suggestions`   — self-healing error suggestions (port from wai)
//! - `suite_linter`  — suite-wide config lint checks (new)

pub mod aix;
pub mod cli;
pub mod config;
pub mod discovery;
pub mod doctor;
pub mod envelope;
pub mod feedback;
pub mod fixture;
pub mod guide;
pub mod managed_block;
pub mod scaffold;
pub mod status;
pub mod suggestions;
pub mod suite_linter;
