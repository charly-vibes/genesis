//! `DoctorSummary` tests.

use super::*;

#[test]
fn test_summary_from_checks() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::warn("b", "d", "warn"),
        CheckEntry::fail("c", "d", "fail", None),
    ];
    let summary = DoctorSummary::from_checks(&checks);
    assert_eq!(summary.pass, 1);
    assert_eq!(summary.warn, 1);
    assert_eq!(summary.fail, 1);
}

#[test]
fn test_summary_is_healthy() {
    assert!(
        DoctorSummary {
            pass: 1,
            warn: 0,
            fail: 0
        }
        .is_healthy()
    );
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 1,
            fail: 0
        }
        .is_healthy()
    );
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 0,
            fail: 1
        }
        .is_healthy()
    );
}

#[test]
fn test_summary_has_failures() {
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 1,
            fail: 0
        }
        .has_failures()
    );
    assert!(
        DoctorSummary {
            pass: 0,
            warn: 0,
            fail: 1
        }
        .has_failures()
    );
}
