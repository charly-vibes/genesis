//! Doctor framework — structured diagnostics with auto-fix support.
//!
//! Extends the [`crate::suite_linter`] pattern with a full doctor CLI workflow:
//! checks with auto-fix, structured reports, JSON envelope serialization,
//! and exit-code handling.
//!
//! ## Design
//!
//! - [`DoctorCheck`] trait — checks that can optionally auto-fix
//! - [`DoctorRunner`] — runs a collection of checks, handles `--fix`
//! - [`DoctorReport`] — structured result with summary, serializable
//! - Uses [`LintResult`](crate::suite_linter::LintResult) and
//!   [`Severity`](crate::suite_linter::Severity) from
//!   [`crate::suite_linter`]
//!
//! ## Usage
//!
//! ```rust
//! use genesis::doctor::{DoctorCheck, DoctorRunner, DoctorReport};
//! use genesis::suite_linter::{LintResult, Severity};
//! use std::path::Path;
//!
//! struct MyCheck;
//! impl DoctorCheck for MyCheck {
//!     fn name(&self) -> &'static str { "my-check" }
//!     fn description(&self) -> &'static str { "Checks something" }
//!     fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
//!         Ok(vec![]) // pass
//!     }
//! }
//!
//! let runner = DoctorRunner::new(vec![Box::new(MyCheck)]);
//! let report = runner.run(&std::env::current_dir().unwrap(), false).unwrap();
//! println!("pass={} warn={} fail={}", report.summary.pass, report.summary.warn, report.summary.fail);
//! ```

mod check;
mod report;
mod runner;

pub use check::{lint_to_doctor, DoctorCheck, LintCheckAdapter};
pub use report::{CheckEntry, CheckStatus, DoctorReport, DoctorSummary};
pub use runner::{runner, DoctorRunner};

#[cfg(test)]
mod tests;
