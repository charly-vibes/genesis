#!/usr/bin/env python3
"""Check every relative link in the published research tree.

Purpose: catch a published page that points outside `docs/public/` before it
ships (pattern: charly-vibes/kaya scripts/check-links.py @ v0.4.1, adapted to
walk the kaya publication's markdown instead of the mdbook build's HTML).
Responsibilities: walk `docs/public/**/*.md`, resolve every relative link
target against its page, and fail naming each unresolved target. Inline
markdown links only — reference-style links and angle-bracket targets are
not parsed (the kaya publication emits inline links).
Rationale: the check is internal-only — no network access, no markdown parser
dependency — and it judges exactly the tree `kaya publish` composes.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)[^)]*\)")


def is_external(target: str) -> bool:
    """True for http(s), mailto, scheme-qualified, host-qualified, bare anchors."""
    parts = urlsplit(target)
    if parts.scheme or parts.netloc:
        return True
    return target.startswith(("#",))


def resolve(page: Path, target: str, root: Path) -> Path | None:
    """The file a relative link lands on, fragment stripped, or None."""
    path_part = unquote(target.split("#", 1)[0])
    if not path_part:
        return page
    resolved = (page.parent / path_part).resolve()
    if not resolved.is_relative_to(root.resolve()):
        return None
    return resolved


def build_parser() -> argparse.ArgumentParser:
    """CLI for the link check."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("docs/public"),
        help="published tree to walk",
    )
    return parser


def page_broken(page: Path, root: Path) -> tuple[list[str], int]:
    """Unresolved relative links on one page, plus relative targets checked."""
    broken: list[str] = []
    checked = 0
    for target in LINK.findall(page.read_text(encoding="utf-8")):
        if is_external(target):
            continue
        checked += 1
        resolved = resolve(page, target, root)
        if resolved is None or not resolved.exists():
            broken.append(f"{page}: {target}")
    return broken, checked


def broken_links(root: Path) -> tuple[list[str], int]:
    """Unresolved relative links as `page: target` strings, plus checks run."""
    broken: list[str] = []
    checked = 0
    for page in sorted(root.rglob("*.md")):
        page_broken_links, page_checked = page_broken(page, root)
        broken.extend(page_broken_links)
        checked += page_checked
    return broken, checked


def report_broken(broken: list[str]) -> None:
    """Print each unresolved link to stderr."""
    print(f"link check: {len(broken)} unresolved link(s):", file=sys.stderr)
    for entry in broken:
        print(f"  {entry}", file=sys.stderr)


def main() -> int:
    args = build_parser().parse_args()

    if not args.root.is_dir():
        print(f"link check: no published tree at {args.root}", file=sys.stderr)
        return 2

    broken, checked = broken_links(args.root)
    if broken:
        report_broken(broken)
        return 1
    print(f"link check: ok — {checked} relative link(s) across {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
