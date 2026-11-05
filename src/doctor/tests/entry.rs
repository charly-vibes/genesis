//! `CheckEntry` builder tests.

use super::*;

#[test]
fn test_check_entry_pass_builder() {
    let e = CheckEntry::pass("test", "description", "all good");
    assert_eq!(e.name, "test");
    assert_eq!(e.status, CheckStatus::Pass);
    assert!(e.fix.is_none());
}

#[test]
fn test_check_entry_warn_builder() {
    let e = CheckEntry::warn("test", "description", "looks off");
    assert_eq!(e.status, CheckStatus::Warn);
}

#[test]
fn test_check_entry_fail_builder() {
    let e = CheckEntry::fail("test", "description", "broken", Some("fix it".into()));
    assert_eq!(e.status, CheckStatus::Fail);
    assert_eq!(e.fix, Some("fix it".into()));
}

#[test]
fn test_check_entry_fail_without_fix() {
    let e = CheckEntry::fail("test", "description", "broken", None);
    assert_eq!(e.status, CheckStatus::Fail);
    assert!(e.fix.is_none());
}
