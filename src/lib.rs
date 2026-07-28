//! genesis — shared crate for cross-cutting CLI/AIX/self-healing infrastructure.
//!
//! Six modules, each extracted only when >=2 tools need it:
//!
//! - `envelope`     — structured CLI output envelope (port from dont)
//! - `suggestions`  — self-healing error suggestions (port from wai)
//! - `managed_block` — managed block injector (port from wai/dont/espectacular)
//! - `aix`          — AIX artifact generation (port from wai)
//! - `feedback`     — agent issue reporting (new)
//! - `suite_linter` — suite-wide config lint checks (new)

pub mod aix;
pub mod envelope;
pub mod feedback;
pub mod managed_block;
pub mod suggestions;
pub mod suite_linter;
