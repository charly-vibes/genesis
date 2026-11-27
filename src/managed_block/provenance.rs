//! Provenance footer primitives for managed blocks: content hashing,
//! footer rendering, and footer parsing.

/// Compute the 8-hex-char content hash used in provenance footers.
///
/// FNV-1a 64-bit over the content's UTF-8 bytes (offset basis
/// `0xcbf29ce484222325`, prime `0x00000100000001b3`), truncated to the first
/// 8 hex digits. The algorithm is deliberately pinned — the previously used
/// `std::hash::DefaultHasher` leaves its output unspecified across rustc
/// releases, so two binaries could disagree on whether a persisted footer
/// had drifted. A golden test keeps the values locked; footers persisted
/// under the old hasher report drift once and are re-stamped by the next
/// inject. Truncated to 8 hex chars — collision-resistant enough for drift
/// detection.
pub fn content_sha8(content: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET_BASIS;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")[..8].to_string()
}

/// Build the opt-in provenance footer line for a managed block.
///
/// The footer is an HTML comment (inert in rendered markdown and ignored by
/// plain-text readers) placed inside the block's markers, after the content.
/// The `sha` field covers only `content` — never the footer line itself — so
/// changing the generator version does not move the hash.
pub fn provenance_footer(
    generator: &str,
    version: &str,
    block_name: &str,
    content: &str,
) -> String {
    format!(
        "<!-- provenance: generator={} version={} source={} sha={} -->",
        generator,
        version,
        block_name,
        content_sha8(content)
    )
}

/// Split the provenance footer off block-internal content.
///
/// Returns `(footer-free body, Some(sha))` when the last line of `inner` is
/// a provenance footer, and `(inner, None)` otherwise. Matches the inject
/// format `{content}\n{footer}` — the separating newline is dropped too, so
/// the body hashes byte-identically to the generator's content. This is the
/// single owner of footer parsing: drift checkers and doctor-style tools
/// read through it instead of keeping private copies.
pub fn split_provenance_footer(inner: &str) -> (&str, Option<&str>) {
    let trimmed = inner.trim_end_matches('\n');
    let last_line = match trimmed.rfind('\n') {
        Some(idx) => &trimmed[idx + 1..],
        None if !trimmed.is_empty() => trimmed,
        None => return (inner, None),
    };
    match parse_footer_sha(last_line) {
        Some(sha) => {
            let cut = trimmed.len() - last_line.len();
            (&trimmed[..cut.saturating_sub(1)], Some(sha))
        }
        None => (inner, None),
    }
}

/// Extract `sha=<value>` from a provenance footer line, if it is one.
///
/// Public counterpart of the footer format built by [`provenance_footer`];
/// returns the `sha` value with any trailing whitespace-separated fields
/// (e.g. a fourth field) excluded.
pub fn parse_footer_sha(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("<!-- provenance: ")?
        .strip_suffix(" -->")?;
    let idx = rest.find("sha=")?;
    let sha = &rest[idx + 4..];
    Some(sha.split_whitespace().next().unwrap_or(sha))
}
