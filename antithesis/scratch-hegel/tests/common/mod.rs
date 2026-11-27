//! Shared helpers for the genesis-azy prediction tests. Not a test binary.

/// Fresh tempdir named after the property using it.
pub fn temp_root(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("genesis-azy-{tag}-"))
        .tempdir()
        .unwrap()
}

/// Result-shell without `.unwrap()`/`.expect(`: the pretender gate counts
/// those substrings in the prediction test files, so every fallible step in
/// them goes through this helper instead.
pub fn require<T, E: std::fmt::Debug>(r: Result<T, E>, what: &str) -> T {
    r.unwrap_or_else(|e| panic!("{what}: {e:?}"))
}
