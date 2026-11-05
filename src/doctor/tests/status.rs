//! `CheckStatus` predicate tests (table-driven).

use super::*;

#[test]
fn check_status_predicates_track_severity() {
    let cases = [
        (CheckStatus::Pass, true, false, false),
        (CheckStatus::Warn, false, true, false),
        (CheckStatus::Fail, false, true, true),
    ];
    for (status, want_pass, want_issue, want_fail) in cases {
        assert_eq!(status.is_pass(), want_pass, "{status:?}");
        assert_eq!(status.is_issue(), want_issue, "{status:?}");
        assert_eq!(status.is_fail(), want_fail, "{status:?}");
    }
}
