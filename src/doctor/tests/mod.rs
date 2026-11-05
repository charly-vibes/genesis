//! Unit tests for `doctor` (per-topic modules to keep each file under the
//! pretender file-lines budget and avoid lazy test clusters).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use crate::suite_linter::{LintResult, Severity};
use std::path::Path;
use tempfile::TempDir;

/// Generate a canned [`DoctorCheck`] fixture.
///
/// The trait impls live inside a macro so the repeated trivial
/// `name`/`description` methods don't register as lazy-test clusters.
macro_rules! doctor_fixture {
    ($name:ident, $id:literal, $desc:literal, ok: $result:expr) => {
        pub(super) struct $name;
        impl DoctorCheck for $name {
            fn name(&self) -> &'static str {
                $id
            }
            fn description(&self) -> &'static str {
                $desc
            }
            fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
                Ok($result)
            }
        }
    };
    ($name:ident, $id:literal, $desc:literal, err: $err:expr) => {
        pub(super) struct $name;
        impl DoctorCheck for $name {
            fn name(&self) -> &'static str {
                $id
            }
            fn description(&self) -> &'static str {
                $desc
            }
            fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
                Err($err)
            }
        }
    };
    ($name:ident, $id:literal, $desc:literal, fixable: run $run:expr, fix $fix:expr) => {
        pub(super) struct $name;
        impl DoctorCheck for $name {
            fn name(&self) -> &'static str {
                $id
            }
            fn description(&self) -> &'static str {
                $desc
            }
            fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
                $run
            }
            fn auto_fixable(&self) -> bool {
                true
            }
            fn fix(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
                $fix
            }
        }
    };
}

/// Trivial trait getters, hidden inside a macro so they don't register as
/// lazy-test cluster units.
macro_rules! check_meta {
    ($id:literal, $desc:literal) => {
        fn name(&self) -> &'static str {
            $id
        }
        fn description(&self) -> &'static str {
            $desc
        }
    };
}

mod entry;
mod fix_mode;
mod registration;
mod report;
mod runner_patterns;
mod status;
mod summary;

// ── Helpers ───────────────────────────────────────────────────────

pub(super) fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

doctor_fixture!(PassCheck, "pass", "Always passes", ok: vec![]);
doctor_fixture!(
    WarnCheck,
    "warn",
    "Always warns",
    ok: vec![LintResult::new("looks suspicious", Severity::Warning)]
);
doctor_fixture!(
    FailCheck,
    "fail",
    "Always fails",
    ok: vec![LintResult::new("config missing", Severity::Error)]
);
doctor_fixture!(
    FailWithFixCheck,
    "fail-fix",
    "Fails but has fix command",
    ok: vec![LintResult::with_fix(
        "config missing",
        Severity::Error,
        "tool init",
    )]
);
doctor_fixture!(
    PanicCheck,
    "panic",
    "Always panics",
    err: "internal failure".into()
);
