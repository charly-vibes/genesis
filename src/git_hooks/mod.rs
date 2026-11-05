//! Git hook primitives shared across the suite.
//!
//! Consolidates overlapping git-hook mechanics from three donors:
//!
//! - pretender (`src/main.rs`): install/uninstall with ownership marker,
//!   refuse-foreign-hook guard, `repo_root()` parent walk.
//! - wai (`src/commands/way/hooks.rs`): `core.hooksPath` resolution,
//!   hook-read helpers.
//! - espectacular (`src/init.rs`): lefthook framework detection and
//!   managed-block injection (rebuilt on [`crate::managed_block`]).
//!
//! Boundary note: this module contains only git-hook mechanics — no
//! consuming-tool gate commands. Callers pass their own markers, gate
//! commands, and block contents as parameters; a guard test enforces
//! this boundary.

mod error;
mod hook;
mod hooks_dir;
mod lifecycle;
mod root;

pub use error::GitHooksError;
pub use hook::{framework, owner, Framework, HookName, Owner};
pub use hooks_dir::{effective_hooks_dir, resolve_hooks_dir, EffectiveHooksDir, HooksDirScope};
pub use lifecycle::{install, uninstall};
pub use root::{repo_root, repo_root_from};

// Test seams: `*_with_env` variants are crate-visible for tests.
#[cfg(test)]
pub(crate) use hook::{framework_with_env, owner_with_env};
#[cfg(test)]
pub(crate) use hooks_dir::{effective_hooks_dir_with_env, resolve_hooks_dir_with_env};
#[cfg(test)]
pub(crate) use lifecycle::{install_with_env, uninstall_with_env};

pub mod lefthook;

#[cfg(test)]
mod tests;
