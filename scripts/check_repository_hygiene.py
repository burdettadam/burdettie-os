#!/usr/bin/env python3
"""Reject accidentally tracked build outputs and local-only inputs."""

from pathlib import Path
import json
import re
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

    # Gitleaks treats long hexadecimal strings as potential API keys. This
    # vendored manifest is a checksum inventory, not a credential store.
    # Validate its schema before the path-scoped Gitleaks allowlist exempts
    # only 64-character lowercase SHA-256 values in this exact file.
    manifest_path = root / "vendor/cosmic-text-upstream-files.json"
    if manifest_path.is_file():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            file_hashes = manifest.get("upstream_files_sha256")
            if not isinstance(file_hashes, dict):
                raise ValueError("upstream_files_sha256 must be an object")
            digests = [manifest.get("crate_sha256"), *file_hashes.values()]
            if not digests or any(
                not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None
                for value in digests
            ):
                raise ValueError("all manifest digests must be 64 lowercase hexadecimal characters")
        except (OSError, json.JSONDecodeError, TypeError, ValueError) as error:
            problems.append(
                f"vendor/cosmic-text-upstream-files.json: invalid checksum manifest: {error}"
            )
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
