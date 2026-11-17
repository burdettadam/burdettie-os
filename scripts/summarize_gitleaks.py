#!/usr/bin/env python3
"""Print only validated location metadata from a private Gitleaks JSON report.

Never print Match, Secret, commit messages, authors, or raw parser exceptions.
This does not evaluate or suppress findings; the caller preserves scanner exit.
"""
from __future__ import annotations

import json
from pathlib import Path, PurePosixPath
import re
import sys
from typing import Any

MAX_BYTES = 8 * 1024 * 1024
MAX_FINDINGS = 10_000


class InvalidReport(ValueError):
    """A report cannot safely be summarized."""


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise InvalidReport("duplicate field")
        result[key] = value
    return result


def summarize(raw: bytes) -> dict[str, Any]:
    if len(raw) > MAX_BYTES:
        raise InvalidReport("report too large")
    try:
        findings = json.loads(raw, object_pairs_hook=unique_object)
    except (ValueError, UnicodeError, RecursionError) as exc:
        raise InvalidReport("invalid JSON") from exc
    if not isinstance(findings, list) or len(findings) > MAX_FINDINGS:
        raise InvalidReport("invalid finding list")
    locations = []
    for finding in findings:
        if not isinstance(finding, dict):
            raise InvalidReport("invalid finding")
        rule = finding.get("RuleID")
        commit = finding.get("Commit")
        file = finding.get("File")
        first = finding.get("StartLine")
        last = finding.get("EndLine")
        if not isinstance(rule, str) or re.fullmatch(r"[a-zA-Z0-9_-]{1,128}", rule) is None:
            raise InvalidReport("invalid rule")
        if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
            raise InvalidReport("invalid commit")
        if (
            not isinstance(file, str)
            or not file
            or len(file) > 2048
            or not file.isascii()
            or any(ord(char) < 32 or ord(char) == 127 for char in file)
            or "\\" in file
            or PurePosixPath(file).is_absolute()
            or ".." in PurePosixPath(file).parts
        ):
            raise InvalidReport("invalid file")
        if type(first) is not int or type(last) is not int or first < 1 or last < first:
            raise InvalidReport("invalid line range")
        locations.append({"rule": rule, "commit": commit, "file": file, "first_line": first, "last_line": last})
    return {"finding_count": len(locations), "locations": locations}


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if len(arguments) != 1:
        print("Usage: summarize_gitleaks.py REPORT", file=sys.stderr)
        return 2
    try:
        with Path(arguments[0]).open("rb") as stream:
            result = summarize(stream.read(MAX_BYTES + 1))
    except (OSError, InvalidReport):
        # Exception text and the supplied path may contain sensitive material.
        print("Gitleaks location summary unavailable: report is missing or invalid.", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
