//! Provenance-footer tests for `managed_block` (add-artifact-provenance §1).

// pretender: role=test

use super::{
    BlockInjector, InjectResult, content_sha8, parse_footer_sha, provenance_footer,
    split_provenance_footer, test_injector, test_registry, tmp,
};
use crate::testutil::TestUnwrap;

/// Extract the sha8 field from a provenance footer line.
fn footer_sha(line: &str) -> &str {
    let rest = line.split("sha=").nth(1).require("footer has sha=");
    rest.split(' ').next().require("sha terminated by space")
}

/// Locate the provenance footer line in injected file content.
fn footer_line(content: &str) -> &str {
    content
        .lines()
        .find(|l| l.contains("provenance:"))
        .require("footer line present")
}

/// Byte offsets of (start marker, footer, end marker) in a provenance injection.
fn provenance_offsets(content: &str) -> (usize, usize, usize) {
    let start = content.find("<!-- WAI:START -->").require("value");
    let footer = content.find("provenance:").require("value");
    let end = content.find("<!-- WAI:END -->").require("value");
    (start, footer, end)
}

/// RED 1.1: footer-less injection is byte-identical to pre-footer output.
#[test]
fn test_footerless_injection_byte_identical_to_golden() {
    let dir = tmp();
    let path = dir.path().join("golden.md");
    let injector = test_injector();
    injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(content, "<!-- WAI:START -->\n# Content\n<!-- WAI:END -->");
    assert!(!content.contains("provenance:"));
}

/// RED 1.2: opt-in provenance footer written inside the markers, stamped
/// with the generator's *own* version — not genesis's `CARGO_PKG_VERSION`.
#[test]
fn test_provenance_footer_written_inside_markers() {
    let dir = tmp();
    let path = dir.path().join("prov.md");
    let injector = BlockInjector::new(test_registry()).with_provenance("genesis", "1.2.3");
    injector
        .inject(&path, "WAI", "\n# Content\n")
        .require("value");
    let content = std::fs::read_to_string(&path).require("value");

    let footer = footer_line(&content);
    assert!(footer.starts_with("<!-- ") && footer.ends_with(" -->"));
    assert!(footer.contains("generator=genesis"));
    assert!(footer.contains("version=1.2.3"));
    assert!(
        !footer.contains(env!("CARGO_PKG_VERSION")),
        "footer must not carry genesis's own version: {footer}"
    );
    assert!(footer.contains("source=WAI"));
    assert!(footer.contains(&format!("sha={}", content_sha8("\n# Content\n"))));

    let (start, fpos, end) = provenance_offsets(&content);
    assert!(start < fpos && fpos < end);
}

/// Inject `content` into `path` and return the footer sha of the result.
fn injected_sha(path: &std::path::Path, injector: &BlockInjector, content: &str) -> String {
    injector.inject(path, "WAI", content).require("value");
    footer_sha(footer_line(&std::fs::read_to_string(path).require("value"))).to_string()
}

/// RED 1.3a: footer round-trips through re-inject with updated hash.
#[test]
fn test_footer_round_trip_updates_hash() {
    let dir = tmp();
    let path = dir.path().join("rt.md");
    let injector = BlockInjector::new(test_registry()).with_provenance("genesis", "1.2.3");

    let first_sha = injected_sha(&path, &injector, "\n# Old\n");

    let result = injector.inject(&path, "WAI", "\n# New\n").require("value");
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).require("value");
    let second_sha = footer_sha(footer_line(&content));
    assert_ne!(first_sha, second_sha);
    assert_eq!(second_sha, content_sha8("\n# New\n"));
    assert!(injector.has_block(&path, "WAI"));
    assert_eq!(content.matches("provenance:").count(), 1);
}

/// RED 1.3b: content hash excludes the footer line itself — version
/// changes do not move the hash for identical content.
#[test]
fn test_footer_hash_excludes_footer_line_and_version() {
    let c = "\n# Same content\n";
    let f_v1 = provenance_footer("genesis", "0.1.0", "WAI", c);
    let f_v2 = provenance_footer("genesis", "9.9.9", "WAI", c);
    assert_ne!(f_v1, f_v2, "footer text carries the version");
    assert_eq!(footer_sha(&f_v1), footer_sha(&f_v2));
    assert_eq!(footer_sha(&f_v1), content_sha8(c));
}

// ── Persisted-sha stability ────────────────────────────────────────

/// Golden: `content_sha8` is FNV-1a 64-bit (offset basis `0xcbf29ce484222325`,
/// prime `0x00000100000001b3`) truncated to the first 8 hex digits. The
/// values below are published FNV-1a test vectors — unlike the previously
/// used `std::hash::DefaultHasher`, whose output is unspecified across
/// rustc releases, these survive a toolchain bump, which is what lets
/// separate binaries agree on whether a persisted footer drifted.
#[test]
fn test_content_sha8_is_pinned_fnv1a_golden() {
    assert_eq!(content_sha8(""), "cbf29ce4", "offset basis, truncated");
    assert_eq!(content_sha8("a"), "af63dc4c");
    assert_eq!(content_sha8("hello"), "a430d846");
    assert_eq!(content_sha8("foobar"), "85944171");
}

// ── Shared footer reader ───────────────────────────────────────────

/// The exposed reader splits injected output back to the hashed body.
#[test]
fn test_split_provenance_footer_recovers_body_and_sha() {
    let content = "\n# Block\n";
    let footer = provenance_footer("genesis", "1.2.3", "WAI", content);
    let inner = format!("{content}\n{footer}");
    let (body, sha) = split_provenance_footer(&inner);
    assert_eq!(body, content, "separating newline dropped");
    assert_eq!(sha, Some(content_sha8(content).as_str()));
}

/// Footer-less inner content is returned unchanged, with no sha.
#[test]
fn test_split_provenance_footer_without_footer() {
    let inner = "\n# Block\n<!-- some other comment -->";
    let (body, sha) = split_provenance_footer(inner);
    assert_eq!(body, inner, "unchanged when no footer");
    assert_eq!(sha, None);
}

/// `parse_footer_sha` reads a footer line and only a footer line.
#[test]
fn test_parse_footer_sha_reads_sha_field_only() {
    let line = "<!-- provenance: generator=genesis version=1.2.3 source=WAI sha=abcd1234 -->";
    assert_eq!(parse_footer_sha(line), Some("abcd1234"));
    assert_eq!(parse_footer_sha("<!-- not a footer -->"), None);
}
