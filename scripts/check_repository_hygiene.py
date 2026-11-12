#!/usr/bin/env python3
"""Reject accidentally tracked build outputs and local-only inputs."""

from pathlib import Path
import subprocess
import sys


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args])


def main() -> int:
    root = Path(git("rev-parse", "--show-toplevel").decode().strip())
    tracked = [p for p in git("ls-files", "-z").decode().split("\0") if p]
    ignored = {
        p
        for p in git("ls-files", "-ci", "--exclude-standard", "-z").decode().split("\0")
        if p
    }
    problems = [f"{p}: tracked despite an ignore rule" for p in sorted(ignored)]
    synthetic_inputs = {"fixtures/malformed/bad_magic", "fixtures/malformed/truncated_header"}
    executable_magic = {
        b"\x7fELF",
        b"\xfe\xed\xfa\xce", b"\xce\xfa\xed\xfe",
        b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca",
        b"\xca\xfe\xba\xbf", b"\xbf\xba\xfe\xca",
    }
    for relative in tracked:
        path = root / relative
        if path.is_symlink() or not path.is_file():
            continue
        if path.stat().st_size > 5 * 1024 * 1024:
            problems.append(f"{relative}: exceeds the 5 MiB source-repository limit")
        with path.open("rb") as stream:
            prefix = stream.read(4)
        if relative not in synthetic_inputs and (prefix in executable_magic or prefix[:2] == b"MZ"):
            problems.append(f"{relative}: compiled executable or library; retain its source/build recipe")
    if problems:
        print("Repository hygiene failed:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    print(f"Repository hygiene passed for {len(tracked)} tracked files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
