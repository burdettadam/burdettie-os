#!/usr/bin/env python3
"""Check resolved Darwin CLI feature graphs without compiling or using an SDK."""

import argparse
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
DIALOG_PACKAGES = {"rfd", "objc2-app-kit"}
WINDOW_PACKAGES = {"window-backend-darwin", "winit", "softbuffer"}
REQUIRED_PACKAGES = {"macho-inspect", "macho-runtime", "cf-lite"}


def check_graph(target, offline, name, feature_args, present, absent):
    command = [
        "cargo", "tree", "--locked", "--target", target,
        "-p", "macho-inspect", "--edges", "normal,build",
        "--prefix", "none", "--format", "{p}", *feature_args,
    ]
    if offline:
        command.append("--offline")
    result = subprocess.run(
        command, cwd=ROOT, text=True, capture_output=True, timeout=120, check=True,
    )
    packages = {line.split()[0] for line in result.stdout.splitlines() if line.strip()}
    missing = (REQUIRED_PACKAGES | present) - packages
    unexpected = absent & packages
    if missing or unexpected:
        raise ValueError(
            f"{name}: missing packages {sorted(missing)}; "
            f"unexpected packages {sorted(unexpected)}"
        )
    print(f"{name}: pass ({len(packages)} resolved package names, target={target})")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default="x86_64-apple-darwin",
                        choices=("x86_64-apple-darwin", "aarch64-apple-darwin"))
    parser.add_argument("--offline", action="store_true",
                        help="require the locked Cargo inputs to be cached")
    args = parser.parse_args()
    cases = (
        ("headless", ["--no-default-features"], set(), DIALOG_PACKAGES | WINDOW_PACKAGES),
        ("default", [], DIALOG_PACKAGES, WINDOW_PACKAGES),
        ("explicit-gui", ["--no-default-features", "--features", "gui"],
         DIALOG_PACKAGES | WINDOW_PACKAGES, set()),
    )
    try:
        for name, features, present, absent in cases:
            check_graph(args.target, args.offline, name, features, present, absent)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"dependency check failed: {exc}", file=sys.stderr)
        if isinstance(exc, subprocess.CalledProcessError):
            print(exc.stderr, file=sys.stderr)
        return 1
    print("Dependency selection only; target compilation and guest execution are not tested.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
