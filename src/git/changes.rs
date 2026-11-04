//! Changed/uncommitted file enumeration and porcelain parsing.

use std::path::Path;

use super::GitError;
use super::runner::{EnvPolicy, is_unborn_head, lines_of, run};

/// Enumerate untracked files (git's own ignore rules respected).
fn untracked_files_with_policy(root: &Path, policy: EnvPolicy) -> Result<Vec<String>, GitError> {
    let out = run(
        root,
        policy,
        &["ls-files", "--others", "--exclude-standard"],
    )?;
    Ok(lines_of(&out.stdout))
}

/// Files with tracked changes against `HEAD` (staged or unstaged).
///
/// In a repository with no commits (unborn HEAD, where `git diff HEAD`
/// exits 128) this falls back to enumerating untracked files rather than
/// returning an empty set — the unborn-HEAD fallback is the only mode in
/// which untracked files appear here.
pub fn changed_files(root: &Path) -> Result<Vec<String>, GitError> {
    changed_files_with_policy(root, EnvPolicy::Inherit)
}

/// [`changed_files`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn changed_files_with_policy(root: &Path, policy: EnvPolicy) -> Result<Vec<String>, GitError> {
    match run(root, policy, &["diff", "--name-only", "HEAD"]) {
        Ok(out) => Ok(lines_of(&out.stdout)),
        Err(GitError::Git {
            code: 128, stderr, ..
        }) if is_unborn_head(&stderr) => untracked_files_with_policy(root, policy),
        Err(other) => Err(other),
    }
}

/// Files with tracked changes between two revisions (both must resolve).
pub fn changed_files_between(root: &Path, base: &str, head: &str) -> Result<Vec<String>, GitError> {
    changed_files_between_with_policy(root, base, head, EnvPolicy::Inherit)
}

/// [`changed_files_between`] with an explicit environment policy
/// (policy-aware variant; see [`run`] for what the policy controls).
pub fn changed_files_between_with_policy(
    root: &Path,
    base: &str,
    head: &str,
    policy: EnvPolicy,
) -> Result<Vec<String>, GitError> {
    let out = run(root, policy, &["diff", "--name-only", base, head])?;
    Ok(lines_of(&out.stdout))
}

/// Every entry of `git status --porcelain` (v1): staged, unstaged,
/// renamed (new path), and untracked files.
pub fn uncommitted_files(root: &Path) -> Result<Vec<String>, GitError> {
    uncommitted_files_with_policy(root, EnvPolicy::Inherit)
}

/// [`uncommitted_files`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn uncommitted_files_with_policy(
    root: &Path,
    policy: EnvPolicy,
) -> Result<Vec<String>, GitError> {
    let out = run(root, policy, &["status", "--porcelain"])?;
    Ok(parse_porcelain(&String::from_utf8_lossy(&out.stdout)))
}

/// Like [`uncommitted_files`], but degrades silently: on any git failure
/// (non-repo directory, spawn failure, non-zero exit) returns an empty
/// set instead of an error. Use only where an empty result is an
/// acceptable answer; the degradation is contractual and explicit.
pub fn uncommitted_files_lossy(root: &Path) -> Vec<String> {
    uncommitted_files(root).unwrap_or_default()
}

/// Like [`changed_files`], but degrades silently: on any git failure
/// (non-repo directory, spawn failure, non-zero exit) returns an empty
/// set instead of an error. Symmetric with [`uncommitted_files_lossy`];
/// use only where an empty result is an acceptable answer — the
/// degradation is contractual and explicit.
pub fn changed_files_lossy(root: &Path) -> Vec<String> {
    changed_files(root).unwrap_or_default()
}

/// Parse a `git status --porcelain` (v1) body into file paths.
///
/// Plain `XY path` records pass through; rename/copy records
/// `XY old -> new` report only the new path (never the joined string);
/// quoted paths (`core.quotePath`) are unquoted, including C-style
/// escapes (`\"`, `\\`, `\t`, `\n`, octal `\NNN` byte values); an
/// empty body parses to an empty list.
pub fn parse_porcelain(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| is_porcelain_record(line))
        .filter_map(|line| porcelain_path(&line[3..], line.as_bytes()))
        .filter(|path| !path.is_empty())
        .collect()
}

/// A porcelain record needs at least two status chars, a space, and a path.
fn is_porcelain_record(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 4 && bytes[2] == b' '
}

/// The reported path of a porcelain record: the new path for renames/copies
/// (staged `X` or worktree column `Y`), C-style unquoted when quoted.
fn porcelain_path(raw: &str, bytes: &[u8]) -> Option<String> {
    let is_rename_or_copy =
        bytes[0] == b'R' || bytes[0] == b'C' || bytes[1] == b'R' || bytes[1] == b'C';
    let path_part = if is_rename_or_copy {
        raw.rsplit_once(" -> ").map(|(_old, new)| new).unwrap_or(raw)
    } else {
        raw
    };
    let path = if path_part.len() >= 2 && path_part.starts_with('"') && path_part.ends_with('"') {
        unquote_c_style(&path_part[1..path_part.len() - 1])
    } else {
        path_part.to_string()
    };
    Some(path)
}

/// Decode git's C-style path quoting (the content between the surrounding
/// double quotes) byte by byte. Octal escapes are raw byte values, so the
/// decoded bytes are lossily converted to UTF-8 at the end.
fn unquote_c_style(inner: &str) -> String {
    let bytes = inner.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let (byte, consumed) = decode_escape(bytes, i + 1);
            out.push(byte);
            i += 1 + consumed;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Decode one escape sequence starting at `bytes[start]` (the char after the
/// backslash). Returns the decoded byte and how many bytes were consumed.
fn decode_escape(bytes: &[u8], start: usize) -> (u8, usize) {
    let simple = |byte: u8| (byte, 1);
    match bytes[start] {
        b'"' => simple(b'"'),
        b'\'' => simple(b'\''),
        b'\\' => simple(b'\\'),
        b't' => simple(b'\t'),
        b'n' => simple(b'\n'),
        b'r' => simple(b'\r'),
        b'a' => simple(0x07),
        b'b' => simple(0x08),
        b'f' => simple(0x0C),
        b'v' => simple(0x0B),
        digit @ b'0'..=b'7' => octal_escape(bytes, start, digit),
        other => (other, 1),
    }
}

/// Decode up to three octal digits (first at `start`) into one byte.
/// Octal escapes are raw byte values, not UTF-8 code points.
fn octal_escape(bytes: &[u8], start: usize, first: u8) -> (u8, usize) {
    let digits: Vec<u8> = std::iter::once(first)
        .chain(
            bytes
                .iter()
                .skip(start + 1)
                .copied()
                .take(2)
                .take_while(|b| (b'0'..=b'7').contains(b)),
        )
        .collect();
    let value = digits
        .iter()
        .fold(0u32, |acc, b| acc * 8 + u32::from(b - b'0')) as u8;
    (value, digits.len())
}

/// Path queries run with `root` as the working directory, so git needs
/// path arguments relative to `root`. Callers may pass absolute paths
/// under `root`; those are relativized, anything else passes through.
pub(super) fn relative_to(root: &Path, path: &Path) -> String {
    if let Ok(rest) = path.strip_prefix(root) {
        rest.to_string_lossy().into_owned()
    } else {
        path.to_string_lossy().into_owned()
    }
}
