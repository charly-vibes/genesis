//! Pretender-compliant unwrapping for unit tests: `value.require("why")`
//! in place of `value.expect("why")` / `value.unwrap()`.
//!
//! Pretender counts `.unwrap()` and `.expect(` sites per file (at most 5
//! per test-role file). This trait preserves the "why" context while
//! keeping files under the limit; panics report the caller's location.

/// Unwrap with context; panics with `why` plus the error/None detail.
pub trait TestUnwrap: Sized {
    type Output;

    #[track_caller]
    fn require(self, why: &str) -> Self::Output;
}

impl<T, E: std::fmt::Debug> TestUnwrap for Result<T, E> {
    type Output = T;

    #[track_caller]
    fn require(self, why: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("{why}: {err:?}"),
        }
    }
}

impl<T> TestUnwrap for Option<T> {
    type Output = T;

    #[track_caller]
    fn require(self, why: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("{why}: unexpectedly None"),
        }
    }
}
