//! Shared config management for suite tools.
//!
//! Provides `ConfigFile` trait, `ConfigRegistry`, `ConfigStore` — each tool
//! just implements the trait and gets read/write/discover/validate for free.
//!
//! ## AIX: self-healing config errors
//!
//! Every config error carries a `Suggestion` footer so the tool's error-sink
//! wiring (the `→ Run: …` pattern) works without changes.

mod error;
mod registry;
mod store;
mod validation;

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "config/tests_store.rs"]
mod tests_store;

pub use error::ConfigError;
pub use registry::{ConfigEntry, ConfigRegistry};
pub use store::{ConfigStore, DiscoveredConfig};
pub use validation::{ConfigFile, ConfigValidation, ValidationSeverity};
