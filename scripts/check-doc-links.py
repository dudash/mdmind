#!/usr/bin/env python3

from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
LINK_RE = re.compile(r"!\[[^\]]*\]\(([^)]+)\)|\[[^\]]+\]\(([^)]+)\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.*)$")
SKIP_PREFIXES = ("http://", "https://", "mailto:")
SKIP_DIRS = {".git", "target"}


def iter_markdown_files() -> list[Path]:
    files: list[Path] = []
    for path in REPO_ROOT.rglob("*.md"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return sorted(files)


def github_anchor(text: str) -> str:
    text = unicodedata.normalize("NFKD", text).lower().strip()
    text = "".join(ch for ch in text if not unicodedata.combining(ch))

    chars: list[str] = []
    for ch in text:
        if ch.isalnum() or ch in {" ", "-", "_"}:
            chars.append(ch)

    anchor = "".join(chars).replace(" ", "-")
    while "--" in anchor:
        anchor = anchor.replace("--", "-")
    return anchor.strip("-")


def anchors_for(markdown_path: Path) -> set[str]:
    anchors: set[str] = set()
    for line in markdown_path.read_text().splitlines():
        match = HEADING_RE.match(line)
        if match:
            anchors.add(github_anchor(match.group(2)))
    return anchors


def resolve_target(source: Path, raw_target: str) -> tuple[Path | None, str | None, bool]:
    if raw_target.startswith(SKIP_PREFIXES) or raw_target.startswith("#"):
        return None, None, False

    path_part, _, fragment = raw_target.partition("#")
    if not path_part:
        return source, fragment or None, False

    candidate = Path(path_part)
    if candidate.is_absolute():
        return candidate, fragment or None, True

    return (source.parent / candidate).resolve(), fragment or None, False


def main() -> int:
    failures: list[str] = []

    for markdown_path in iter_markdown_files():
        lines = markdown_path.read_text().splitlines()
        for lineno, line in enumerate(lines, 1):
            for match in LINK_RE.finditer(line):
                raw_target = match.group(1) or match.group(2)
                if not raw_target:
                    continue

                target, fragment, is_absolute = resolve_target(markdown_path, raw_target.strip())
                if target is None:
                    continue

                rel_source = markdown_path.relative_to(REPO_ROOT)

                if is_absolute:
                    failures.append(
                        f"{rel_source}:{lineno}: absolute local link is not portable: {raw_target}"
                    )
                    continue

                if not target.exists():
                    failures.append(
                        f"{rel_source}:{lineno}: missing target: {raw_target}"
                    )
                    continue

                if fragment and target.suffix.lower() == ".md":
                    anchors = anchors_for(target)
                    if fragment not in anchors:
                        failures.append(
                            f"{rel_source}:{lineno}: missing anchor '#{fragment}' in {target.relative_to(REPO_ROOT)}"
                        )

    if failures:
        print("Doc link check failed:\n", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    print("Doc link check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
