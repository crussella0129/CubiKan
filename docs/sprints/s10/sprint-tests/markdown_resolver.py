#!/usr/bin/env python3
"""Resolve repository-local Markdown links, fragments, and Book navigation."""

from __future__ import annotations

import html
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


LINK_RE = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")
REFERENCE_DEFINITION_RE = re.compile(
    r"^ {0,3}\[([^\]\n]+)\]:\s*(<[^>]+>|\S+)(?:\s+.*)?$"
)
FULL_REFERENCE_RE = re.compile(r"!?\[([^\]\n]+)\]\[([^\]\n]*)\]")
SHORTCUT_REFERENCE_RE = re.compile(r"!?\[([^\]\n]+)\](?![\[(])")
HEADING_RE = re.compile(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$")
FENCE_RE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
EXPLICIT_ANCHOR_RE = re.compile(
    r"<(?:a\s+(?:name|id)|[^>]+\sid)=[\"']([^\"']+)[\"'][^>]*>",
    re.IGNORECASE,
)
EXTERNAL_SCHEMES = {"app", "data", "ftp", "http", "https", "mailto"}
EXPECTED_INTENTS = {f"INT-{number:04d}" for number in range(1, 14)}


def repository_markdown(root: Path) -> list[Path]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
    ).decode("utf-8", errors="strict")
    paths = {
        (root / item).resolve()
        for item in output.split("\0")
        if item.endswith(".md") and (root / item).is_file()
    }
    return sorted(paths)


def destination(raw: str) -> str:
    value = raw.strip()
    if value.startswith("<"):
        close = value.find(">")
        return value[1:close] if close >= 0 else value[1:]
    # Repository paths do not contain spaces. Text after whitespace is a title.
    return value.split(maxsplit=1)[0]


def heading_slug(heading: str) -> str:
    value = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", heading)
    value = re.sub(r"<[^>]+>", "", value)
    value = html.unescape(value).strip().lower().replace("`", "")
    value = re.sub(r"[^\w\- ]", "", value, flags=re.UNICODE)
    return re.sub(r"\s+", "-", value)


def anchors(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    result = {unquote(anchor).lower() for anchor in EXPLICIT_ANCHOR_RE.findall(text)}
    occurrences: dict[str, int] = {}
    in_fence = False
    fence_marker = ""

    for line in text.splitlines():
        fence = FENCE_RE.match(line)
        if fence:
            marker = fence.group(1)[0]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
                fence_marker = ""
            continue
        if in_fence:
            continue
        match = HEADING_RE.match(line)
        if not match:
            continue
        base = heading_slug(match.group(1))
        suffix = occurrences.get(base, 0)
        occurrences[base] = suffix + 1
        result.add(base if suffix == 0 else f"{base}-{suffix}")
    return result


def resolve_target(root: Path, source: Path, raw: str) -> tuple[Path | None, str]:
    decoded = unquote(raw)
    parsed = urlsplit(decoded)
    if parsed.scheme.lower() in EXTERNAL_SCHEMES or raw.startswith("//"):
        return None, ""
    if parsed.scheme:
        return None, ""
    if parsed.path:
        relative = parsed.path.lstrip("/") if parsed.path.startswith("/") else parsed.path
        target = (root / relative) if parsed.path.startswith("/") else (source.parent / relative)
    else:
        target = source
    target = target.resolve()
    if target.is_dir():
        target = target / "README.md"
    return target, unquote(parsed.fragment).lower()


def content_lines(text: str):
    in_fence = False
    fence_marker = ""
    for line_number, line in enumerate(text.splitlines(), start=1):
        fence = FENCE_RE.match(line)
        if fence:
            marker = fence.group(1)[0]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
                fence_marker = ""
            continue
        if in_fence:
            continue
        yield line_number, line


def reference_label(value: str) -> str:
    return re.sub(r"\s+", " ", value.strip()).casefold()


def reference_definitions(text: str) -> tuple[dict[str, str], list[str]]:
    definitions: dict[str, str] = {}
    errors: list[str] = []
    for line_number, line in content_lines(text):
        match = REFERENCE_DEFINITION_RE.match(line)
        if not match:
            continue
        label = reference_label(match.group(1))
        if label in definitions:
            errors.append(f"{line_number}: duplicate reference definition: {match.group(1)}")
            continue
        definitions[label] = destination(match.group(2))
    return definitions, errors


def iter_links(text: str, definitions: dict[str, str]):
    for line_number, line in content_lines(text):
        if REFERENCE_DEFINITION_RE.match(line):
            continue
        occupied: list[tuple[int, int]] = []
        for match in LINK_RE.finditer(line):
            occupied.append(match.span())
            yield line_number, destination(match.group(1)), None
        for match in FULL_REFERENCE_RE.finditer(line):
            occupied.append(match.span())
            label = match.group(2) or match.group(1)
            normalized = reference_label(label)
            raw = definitions.get(normalized)
            if raw is None:
                yield line_number, "", f"undefined reference label: {label}"
            else:
                yield line_number, raw, None
        for match in SHORTCUT_REFERENCE_RE.finditer(line):
            if any(start < match.end() and match.start() < end for start, end in occupied):
                continue
            normalized = reference_label(match.group(1))
            raw = definitions.get(normalized)
            if raw is not None:
                yield line_number, raw, None


def self_test() -> int:
    sample = """\
[guide]: docs/README.md#project-book
[collapsed]: docs/SUMMARY.md

[full reference][guide]
[collapsed][]
[guide]
"""
    definitions, errors = reference_definitions(sample)
    if errors:
        print(f"markdown_resolver self-test: unexpected definition errors: {errors}", file=sys.stderr)
        return 1
    links = list(iter_links(sample, definitions))
    expected = [
        (4, "docs/README.md#project-book", None),
        (5, "docs/SUMMARY.md", None),
        (6, "docs/README.md#project-book", None),
    ]
    if links != expected:
        print(f"markdown_resolver self-test: reference parse mismatch: {links}", file=sys.stderr)
        return 1

    _, duplicate_errors = reference_definitions("[x]: one.md\n[x]: two.md\n")
    undefined = list(iter_links("[broken][missing]\n", {}))
    if len(duplicate_errors) != 1 or undefined != [
        (1, "", "undefined reference label: missing")
    ]:
        print("markdown_resolver self-test: fail-closed cases were accepted", file=sys.stderr)
        return 1
    print(
        "markdown_resolver self-test: full, collapsed, and shortcut references "
        "resolved; invalid forms rejected"
    )
    return 0


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    files = repository_markdown(root)
    errors: list[str] = []
    anchor_cache: dict[Path, set[str]] = {}
    total_links = 0
    local_links = 0
    fragments = 0

    for source in files:
        text = source.read_text(encoding="utf-8")
        definitions, definition_errors = reference_definitions(text)
        for error in definition_errors:
            errors.append(f"{source.relative_to(root)}:{error}")
        for line_number, raw, reference_error in iter_links(text, definitions):
            total_links += 1
            if reference_error is not None:
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: {reference_error}"
                )
                continue
            target, fragment = resolve_target(root, source, raw)
            if target is None:
                continue
            local_links += 1
            try:
                target.relative_to(root)
            except ValueError:
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: target escapes repository: {raw}"
                )
                continue
            if not target.is_file():
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: missing target: {raw}"
                )
                continue
            if not fragment:
                continue
            fragments += 1
            if target.suffix.lower() != ".md":
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: fragment on non-Markdown target: {raw}"
                )
                continue
            target_anchors = anchor_cache.setdefault(target, anchors(target))
            if fragment not in target_anchors:
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: missing fragment: {raw}"
                )

    summary = root / "docs" / "SUMMARY.md"
    intent_targets: set[Path] = set()
    summary_text = summary.read_text(encoding="utf-8")
    summary_definitions, _ = reference_definitions(summary_text)
    for _, raw, reference_error in iter_links(summary_text, summary_definitions):
        if reference_error is not None:
            continue
        target, _ = resolve_target(root, summary, raw)
        if (
            target
            and target.parent == root / "docs" / "intents"
            and target.name.startswith("INT-")
        ):
            intent_targets.add(target.resolve())

    actual_intents = {
        path.resolve()
        for path in (root / "docs" / "intents").glob("INT-*.md")
        if path.is_file()
    }
    actual_ids = {path.name.split("-", 2)[0] + "-" + path.name.split("-", 2)[1] for path in actual_intents}
    if actual_ids != EXPECTED_INTENTS:
        missing = sorted(EXPECTED_INTENTS - actual_ids)
        extra = sorted(actual_ids - EXPECTED_INTENTS)
        if missing:
            errors.append(f"Book is missing expected intent IDs: {', '.join(missing)}")
        if extra:
            errors.append(f"Book contains unexpected intent IDs: {', '.join(extra)}")

    for path in sorted(actual_intents - intent_targets):
        errors.append(f"docs/SUMMARY.md does not navigate to {path.relative_to(root)}")
    for path in sorted(intent_targets - actual_intents):
        errors.append(f"docs/SUMMARY.md navigates to non-intent {path.relative_to(root)}")

    print(
        "markdown_resolver: "
        f"{len(files)} Markdown files, {total_links} links, {local_links} local targets, "
        f"{fragments} fragments, {len(intent_targets)} Book intents; {len(errors)} errors"
    )
    for error in errors:
        print(f"markdown_resolver: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
