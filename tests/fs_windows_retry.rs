//! Integration tests for the Windows sharing-violation retry (add-fs-module
//! task 1.5).
//!
//! The retry logic is exercised through `with_sharing_retry_bounded`, which
//! takes the failing operation as a parameter — the same injection seam the
//! Windows-only `rename` call site uses. Injecting fake operations keeps these
//! tests green on Linux, where sharing violations cannot occur.

use std::io::Error as IoError;
use std::time::Duration;

use genesis::fs::{with_sharing_retry, with_sharing_retry_bounded};

/// Windows sharing violation (`ERROR_SHARING_VIOLATION`, 32) as an `io::Error`,
/// the exact shape a failing rename reports.
fn sharing_violation() -> IoError {
    IoError::from_raw_os_error(32)
}

fn lock_violation() -> IoError {
    IoError::from_raw_os_error(33)
}

/// Scenario: a transient sharing violation is retried.
///
/// A rename that fails with a sharing violation is retried and eventually
/// succeeds; the failure is reported only after the retries are exhausted.
#[test]
fn sharing_violation_rename_is_retried() {
    let mut calls = 0u32;
    let result = with_sharing_retry_bounded(6, Duration::from_millis(1), || {
        calls += 1;
        if calls < 3 {
            Err(sharing_violation().into())
        } else {
            Ok(())
        }
    });
    assert!(
        result.is_ok(),
        "a transient sharing violation must be retried: {result:?}"
    );
    assert_eq!(calls, 3, "the operation must be retried until it succeeds");

    // Both Windows sharing-error codes retry (32 = sharing, 33 = lock).
    let mut calls = 0u32;
    let result = with_sharing_retry_bounded(6, Duration::from_millis(1), || {
        calls += 1;
        if calls < 2 {
            Err(lock_violation().into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_ok());
    assert_eq!(calls, 2);
}

/// Scenario: a non-sharing rename failure is not retried.
#[test]
fn non_sharing_failure_is_not_retried() {
    let mut calls = 0u32;
    let result = with_sharing_retry_bounded(6, Duration::from_millis(1), || {
        calls += 1;
        Err(IoError::from_raw_os_error(2).into()) // ENOENT
    });
    assert!(
        result.is_err(),
        "a non-sharing failure must surface immediately"
    );
    assert_eq!(calls, 1, "a non-sharing failure must not be retried");

    // The unbounded wrapper still fails fast for non-sharing errors.
    let mut calls = 0u32;
    let result: Result<(), genesis::fs::FsError> = with_sharing_retry(|| {
        calls += 1;
        Err(IoError::from_raw_os_error(2).into())
    });
    assert!(result.is_err());
    assert_eq!(calls, 1);
}

/// Scenario: the retry is bounded.
///
/// A rename that keeps failing with a sharing violation stops after a bounded
/// number of attempts and reports the failure instead of retrying forever.
#[test]
fn retry_is_bounded() {
    let mut calls = 0u32;
    let result = with_sharing_retry_bounded(4, Duration::from_millis(1), || {
        calls += 1;
        Err(sharing_violation().into())
    });
    assert!(
        result.is_err(),
        "an exhausted retry must report the failure"
    );
    assert_eq!(
        calls, 4,
        "the retry must stop after the bounded attempt count"
    );

    // The default wrapper is bounded too, and the reported error is the last
    // sharing violation.
    let result: Result<(), genesis::fs::FsError> =
        with_sharing_retry(|| Err(sharing_violation().into()));
    let err = result.expect_err("default wrapper must give up");
    assert!(
        matches!(&err, genesis::fs::FsError::Io(e) if e.raw_os_error() == Some(32)),
        "the original sharing violation must be reported, got {err:?}"
    );
}
